// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

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

    #[allow(clippy::cast_possible_wrap)]
    pub fn configure_overlay(&mut self, output_id: u32, serial: u32, width: u32, height: u32) {
        let (render_w, render_h) = {
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
            (render_w, render_h)
        };

        self.register_configured_output(output_id, render_w, render_h);

        if self.screensaver_mode {
            return;
        }
        self.attach_solid_buffer(output_id, render_w, render_h);
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn update_frame(
        &mut self,
        output_id: u32,
        width: u32,
        height: u32,
        pixels: std::sync::Arc<Vec<u8>>,
    ) {
        if !self.screensaver_mode {
            return;
        }

        let Some(shm) = &self.shm else {
            return;
        };

        let Some(overlay) = self.overlays.get_mut(&output_id) else {
            return;
        };

        if !super::super::buffer::ensure_frame_buffer(
            &mut overlay.buffer,
            shm,
            &self.queue,
            width,
            height,
            &pixels,
        ) {
            return;
        }

        if !Self::commit_frame_buffer(overlay, width, height) {
            tracing::error!(
                output_id,
                "wayland-present: frame buffer missing after ensure; skipping frame"
            );
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
}
