// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_compositor, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface,
};
use wayland_client::QueueHandle;
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

use crate::appearance::OverlayAppearance;
use crate::output::{OutputLayout, OutputRegistry};

use super::buffer::MappedBuffer;

pub struct OutputTarget {
    pub id: u32,
    pub output: wl_output::WlOutput,
}

pub struct MonitorOverlay {
    pub surface: wl_surface::WlSurface,
    pub layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    pub width: u32,
    pub height: u32,
    pub buffer: Option<MappedBuffer>,
}

/// Mutable Wayland session state owned by the presenter thread.
pub struct SessionState {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub shm: Option<wl_shm::WlShm>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub seat: Option<wl_seat::WlSeat>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_serial: u32,
    pub outputs: Vec<OutputTarget>,
    pub overlays: HashMap<u32, MonitorOverlay>,
    pub appearance: Option<OverlayAppearance>,
    pub screensaver_mode: bool,
    pub visible: Arc<AtomicBool>,
    pub output_registry: OutputRegistry,
    pub output_refresh_hz: HashMap<u32, u32>,
    pub output_origin: HashMap<u32, (i32, i32)>,
    pub output_mode_size: HashMap<u32, (u32, u32)>,
    pub dismiss_grace_until: Option<Instant>,
    pub queue: QueueHandle<SessionState>,
}

impl SessionState {
    pub fn show_solid(&mut self, appearance: OverlayAppearance) {
        self.screensaver_mode = false;
        self.appearance = Some(appearance);
        self.begin_presentation();
    }

    pub fn show_screensaver(&mut self) {
        self.screensaver_mode = true;
        self.appearance = None;
        self.begin_presentation();
        println!("wayland-present: screensaver surfaces ready for frames");
    }

    pub fn hide_pointer(&mut self, serial: u32) {
        if let Some(pointer) = &self.pointer {
            pointer.set_cursor(serial, None, 0, 0);
        }
    }

    fn begin_presentation(&mut self) {
        self.visible.store(true, Ordering::SeqCst);
        self.dismiss_grace_until = Some(Instant::now() + Duration::from_millis(800));
        self.output_registry.clear();
        if self.pointer_serial != 0 {
            self.hide_pointer(self.pointer_serial);
        }

        let output_ids: Vec<u32> = self.outputs.iter().map(|output| output.id).collect();
        for output_id in output_ids {
            self.create_overlay(output_id);
        }

        println!(
            "wayland-present: showing overlay on {} output(s)",
            self.overlays.len()
        );
    }

    pub fn hide(&mut self) {
        self.appearance = None;
        self.screensaver_mode = false;
        self.visible.store(false, Ordering::SeqCst);
        self.output_registry.clear();

        for (_, overlay) in self.overlays.drain() {
            overlay.layer_surface.destroy();
            overlay.surface.destroy();
        }

        println!("wayland-present: overlay hidden");
    }

    pub fn dismiss_from_input(&mut self) {
        if !self.visible.load(Ordering::SeqCst) {
            return;
        }
        if self
            .dismiss_grace_until
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        println!("wayland-present: dismissed by user input");
        self.hide();
    }

    pub fn create_overlay(&mut self, output_id: u32) {
        if self.overlays.contains_key(&output_id) {
            return;
        }

        let (Some(compositor), Some(layer_shell)) = (&self.compositor, &self.layer_shell) else {
            eprintln!("wayland-present: missing compositor or layer shell");
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

        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_margin(0, 0, 0, 0);
        layer_surface.set_keyboard_interactivity(
            zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
        );
        layer_surface.set_size(0, 0);
        surface.commit();

        self.overlays.insert(
            output_id,
            MonitorOverlay {
                surface,
                layer_surface,
                width: 0,
                height: 0,
                buffer: None,
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
        let (render_w, render_h) = Self::render_dimensions(output_id, width, height, &self.output_mode_size);
        overlay.width = render_w;
        overlay.height = render_h;
        let refresh_rate_hz = self
            .output_refresh_hz
            .get(&output_id)
            .copied()
            .unwrap_or(60);
        let (x, y) = self.output_origin.get(&output_id).copied().unwrap_or((0, 0));
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

        overlay.buffer = super::buffer::create_solid_buffer(
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

        overlay.width = width;
        overlay.height = height;
        if super::buffer::ensure_frame_buffer(
            &mut overlay.buffer,
            shm,
            &self.queue,
            width,
            height,
            &pixels,
        ) {
            let buffer = overlay.buffer.as_ref().expect("frame buffer exists after ensure");
            overlay.surface.attach(Some(&buffer.wl_buffer), 0, 0);
            overlay
                .surface
                .damage_buffer(0, 0, width as i32, height as i32);
            overlay.surface.commit();
        }
    }

    pub fn remove_overlay(&mut self, output_id: u32) {
        if let Some(overlay) = self.overlays.remove(&output_id) {
            overlay.layer_surface.destroy();
            overlay.surface.destroy();
        }
    }

    fn render_dimensions(
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

    fn apply_tiling_margins(
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