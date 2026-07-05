// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use wayland_client::protocol::wl_surface;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::output::OutputLayout;

use super::types::SessionState;

impl SessionState {
    pub fn create_overlay(&mut self, output_id: u32) {
        if self.overlays.contains_key(&output_id) {
            return;
        }

        let (Some(compositor), Some(layer_shell)) = (&self.compositor, &self.layer_shell) else {
            tracing::warn!("wayland-present: missing compositor or layer shell");
            return;
        };

        let output = self
            .outputs
            .iter()
            .find(|target| target.id == output_id)
            .map(|target| &target.output);
        let Some(output) = output else {
            return;
        };

        let surface = compositor.create_surface(&self.queue, output_id);
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(output),
            zwlr_layer_shell_v1::Layer::Overlay,
            "trance".to_string(),
            &self.queue,
            output_id,
        );

        let viewport = self
            .viewporter
            .as_ref()
            .map(|vp| vp.get_viewport(&surface, &self.queue, ()));

        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_margin(0, 0, 0, 0);
        layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
        layer_surface.set_size(0, 0);
        surface.commit();

        self.overlays.insert(
            output_id,
            super::types::MonitorOverlay {
                surface,
                layer_surface,
                width: 0,
                height: 0,
                buffer: None,
                viewport,
            },
        );
    }

    pub fn configure_overlay(&mut self, output_id: u32, serial: u32, width: u32, height: u32) {
        let Some(overlay) = self.overlays.get_mut(&output_id) else {
            return;
        };

        Self::apply_tiling_margins(
            &overlay.layer_surface,
            &overlay.surface,
            output_id,
            width,
            height,
            &self.output_mode_size,
        );
        overlay.layer_surface.ack_configure(serial);
        let (render_w, render_h) =
            Self::render_dimensions(output_id, width, height, &self.output_mode_size);
        overlay.width = render_w;
        overlay.height = render_h;

        if let Some(viewport) = &overlay.viewport {
            viewport.set_destination(render_w as i32, render_h as i32);
        }

        let refresh_rate_hz = self
            .output_refresh_hz
            .get(&output_id)
            .copied()
            .unwrap_or(60);
        let (x, y) = self
            .output_origin
            .get(&output_id)
            .copied()
            .unwrap_or((0, 0));
        self.output_registry.upsert(OutputLayout {
            id: output_id,
            width: render_w,
            height: render_h,
            refresh_rate_hz,
            x,
            y,
        });

        if self.screensaver_mode {
            return;
        }

        let Some(appearance) = self.appearance else {
            return;
        };

        let Some(shm) = &self.shm else {
            return;
        };

        overlay.buffer = super::super::buffer::create_solid_buffer(
            shm,
            &self.queue,
            render_w,
            render_h,
            appearance.color,
        );

        if let Some(buffer) = &overlay.buffer {
            overlay.surface.attach(Some(&buffer.wl_buffer), 0, 0);
            overlay
                .surface
                .damage_buffer(0, 0, render_w as i32, render_h as i32);
            overlay.surface.commit();
        }
    }

    pub fn update_frame(&mut self, output_id: u32, width: u32, height: u32, pixels: Vec<u8>) {
        if !self.screensaver_mode {
            return;
        }

        let Some(shm) = &self.shm else {
            return;
        };

        let Some(overlay) = self.overlays.get_mut(&output_id) else {
            return;
        };

        if super::super::buffer::ensure_frame_buffer(
            &mut overlay.buffer,
            shm,
            &self.queue,
            width,
            height,
            &pixels,
        ) {
            let Some(buffer) = overlay.buffer.as_ref() else {
                tracing::error!(
                    output_id,
                    "wayland-present: frame buffer missing after ensure; skipping frame"
                );
                return;
            };

            let dst_w = if overlay.width > 0 {
                overlay.width
            } else {
                width
            };
            let dst_h = if overlay.height > 0 {
                overlay.height
            } else {
                height
            };
            if let Some(viewport) = &overlay.viewport {
                viewport.set_destination(dst_w as i32, dst_h as i32);
            }

            overlay.surface.attach(Some(&buffer.wl_buffer), 0, 0);
            overlay
                .surface
                .damage_buffer(0, 0, width as i32, height as i32);
            overlay.surface.commit();
        }
    }

    pub fn remove_overlay(&mut self, output_id: u32) {
        if let Some(overlay) = self.overlays.remove(&output_id) {
            if let Some(viewport) = overlay.viewport {
                viewport.destroy();
            }
            overlay.layer_surface.destroy();
            overlay.surface.destroy();
        }
    }

    pub(crate) fn render_dimensions(
        output_id: u32,
        configured_w: u32,
        configured_h: u32,
        mode_sizes: &HashMap<u32, (u32, u32)>,
    ) -> (u32, u32) {
        let Some((native_w, native_h)) = mode_sizes.get(&output_id).copied() else {
            return (configured_w, configured_h);
        };
        (native_w.max(configured_w), native_h.max(configured_h))
    }

    pub(crate) fn apply_tiling_margins(
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        surface: &wl_surface::WlSurface,
        output_id: u32,
        configured_w: u32,
        configured_h: u32,
        mode_sizes: &HashMap<u32, (u32, u32)>,
    ) {
        let Some((native_w, native_h)) = mode_sizes.get(&output_id).copied() else {
            layer_surface.set_margin(0, 0, 0, 0);
            surface.commit();
            return;
        };

        let inset_x = native_w.saturating_sub(configured_w) / 2;
        let inset_y = native_h.saturating_sub(configured_h) / 2;
        if inset_x > 0 || inset_y > 0 {
            layer_surface.set_margin(
                -(inset_y as i32),
                -(inset_x as i32),
                -(inset_y as i32),
                -(inset_x as i32),
            );
        } else {
            layer_surface.set_margin(0, 0, 0, 0);
        }
        surface.commit();
    }
}
