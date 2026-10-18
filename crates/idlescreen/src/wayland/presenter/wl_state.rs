// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Wayland presentation state, surface creation with `wp_viewporter`,
//! protocol object cleanup, and input debounce logic.

use std::time::Instant;

use wayland_client::QueueHandle;
use wayland_client::protocol::{
    wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm,
};
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use wayland_protocols::xdg::shell::client::xdg_wm_base;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use super::{DetectedOutput, PresentationSurface};
use crate::wayland::viewport::SurfaceViewport;

#[allow(clippy::struct_excessive_bools)]
pub struct WaylandState {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub shm: Option<wl_shm::WlShm>,
    pub shm_formats: Vec<wl_shm::Format>,
    pub viewporter: Option<WpViewporter>,
    pub seat: Option<wl_seat::WlSeat>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    pub output: Option<wl_output::WlOutput>,
    pub output_width: u32,
    pub output_height: u32,
    pub output_refresh_mhz: i32,
    pub outputs: Vec<DetectedOutput>,
    pub surfaces: Vec<PresentationSurface>,
    pub configured: bool,
    pub configured_width: u32,
    pub configured_height: u32,
    pub exit_requested: bool,
    pub preview_mode: bool,
    pub tour_mode: bool,
    pub scene_step_delta: i32,
    pub initial_pointer_pos: Option<(f64, f64)>,
    pub start_time: Instant,
    pub dpms: crate::wayland::dpms::DpmsController,
}

impl Default for WaylandState {
    fn default() -> Self {
        Self::new()
    }
}

impl WaylandState {
    #[rustfmt::skip]
    pub fn new() -> Self {
        Self {
            compositor: None, shm: None, shm_formats: Vec::new(), viewporter: None,
            seat: None, keyboard: None, pointer: None, layer_shell: None, xdg_wm_base: None,
            output: None, output_width: 0, output_height: 0, output_refresh_mhz: 0,
            outputs: Vec::new(), surfaces: Vec::new(), configured: false,
            configured_width: 0, configured_height: 0, exit_requested: false,
            preview_mode: false, tour_mode: false, scene_step_delta: 0,
            initial_pointer_pos: None, start_time: Instant::now(),
            dpms: crate::wayland::dpms::DpmsController::new(),
        }
    }

    /// Selects 10-bit deep color (`Argb2101010` / `Xrgb2101010`) when advertised by the compositor,
    /// falling back to `Argb8888`.
    pub fn preferred_shm_format(&self) -> (wl_shm::Format, bool) {
        if self.shm_formats.contains(&wl_shm::Format::Argb2101010) {
            (wl_shm::Format::Argb2101010, true)
        } else if self.shm_formats.contains(&wl_shm::Format::Xrgb2101010) {
            (wl_shm::Format::Xrgb2101010, true)
        } else {
            (wl_shm::Format::Argb8888, false)
        }
    }

    pub fn all_configured(&self) -> bool {
        self.surfaces.is_empty() && self.configured
            || !self.surfaces.is_empty() && self.surfaces.iter().all(|s| s.configured)
    }

    pub fn any_configured(&self) -> bool {
        self.surfaces.is_empty() && self.configured || self.surfaces.iter().any(|s| s.configured)
    }

    pub fn primary_metrics(&self, configured_primary: Option<&str>) -> (u32, u32, i32) {
        if let Some(name) = configured_primary.map(str::trim).filter(|s| !s.is_empty())
            && let Some(out) = self
                .outputs
                .iter()
                .find(|o| o.connector_name.eq_ignore_ascii_case(name) && o.width > 0)
        {
            return (out.width, out.height, out.refresh_mhz);
        }
        if let Some(out) = self
            .outputs
            .iter()
            .filter(|o| o.width > 0 && o.height > 0)
            .max_by_key(|o| u64::from(o.width) * u64::from(o.height))
        {
            return (out.width, out.height, out.refresh_mhz);
        }
        let w = self.surfaces.first().map_or(1920, |s| s.configured_width);
        let h = self.surfaces.first().map_or(1080, |s| s.configured_height);
        (w, h, self.output_refresh_mhz)
    }

    pub fn create_presentation_surfaces(
        &mut self,
        qh: &QueueHandle<Self>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let compositor = self
            .compositor
            .as_ref()
            .ok_or("Wayland compositor not available")?;
        if self.layer_shell.is_none() && self.xdg_wm_base.is_none() {
            return Err("No layer shell or xdg_wm_base available".into());
        }
        let output_targets: Vec<Option<wl_output::WlOutput>> = if self.outputs.is_empty() {
            vec![None]
        } else {
            self.outputs
                .iter()
                .map(|o| Some(o.output.clone()))
                .collect()
        };

        for target_output in output_targets {
            let surface = compositor.create_surface(qh, ());
            let vp_obj = self
                .viewporter
                .as_ref()
                .map(|vp| vp.get_viewport(&surface, qh, ()));
            let viewport = SurfaceViewport::new(vp_obj);
            let (mut layer_surface, mut xdg_surface, mut xdg_toplevel) = (None, None, None);

            if let Some(ls_mgr) = &self.layer_shell {
                let ls = ls_mgr.get_layer_surface(
                    &surface,
                    target_output.as_ref(),
                    zwlr_layer_shell_v1::Layer::Overlay,
                    "idlescreen".into(),
                    qh,
                    (),
                );
                ls.set_anchor(zwlr_layer_surface_v1::Anchor::all());
                ls.set_margin(0, 0, 0, 0);
                ls.set_exclusive_zone(-1);
                ls.set_keyboard_interactivity(
                    zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
                );
                ls.set_size(0, 0);
                surface.commit();
                layer_surface = Some(ls);
            } else if let Some(xdg_wm_base) = &self.xdg_wm_base {
                let xs = xdg_wm_base.get_xdg_surface(&surface, qh, ());
                let tl = xs.get_toplevel(qh, ());
                tl.set_title("idlescreen".to_string());
                tl.set_app_id("idlescreen".to_string());
                tl.set_fullscreen(target_output.as_ref());
                surface.commit();
                (xdg_surface, xdg_toplevel) = (Some(xs), Some(tl));
            }

            let def_m = (
                self.output_width,
                self.output_height,
                self.output_refresh_mhz,
            );
            let (width, height, refresh_mhz) = target_output
                .as_ref()
                .and_then(|o| self.outputs.iter().find(|out| &out.output == o))
                .map_or(def_m, |out| (out.width, out.height, out.refresh_mhz));
            self.surfaces.push(PresentationSurface {
                output: target_output,
                surface,
                viewport,
                layer_surface,
                xdg_surface,
                xdg_toplevel,
                configured: false,
                configured_width: 0,
                configured_height: 0,
                width,
                height,
                refresh_mhz,
                buffers: None,
                current_buf: 0,
                displayed_cells: Vec::new(),
                displayed_subcell_gen: 0,
                needs_commit: true,
            });
        }
        let wl_outputs: Vec<_> = self.outputs.iter().map(|o| o.output.clone()).collect();
        self.dpms.bind_outputs(&wl_outputs, qh);
        Ok(())
    }

    pub fn cleanup_presentation(&mut self, event_queue: &mut wayland_client::EventQueue<Self>) {
        self.dpms.destroy();
        for mut s in self.surfaces.drain(..) {
            s.viewport.destroy();
            if let Some(layer) = s.layer_surface.take() {
                layer.destroy();
            }
            if let Some(toplevel) = s.xdg_toplevel.take() {
                toplevel.destroy();
            }
            if let Some(xdg_surface) = s.xdg_surface.take() {
                xdg_surface.destroy();
            }
            s.surface.destroy();
            drop(s.buffers);
        }
        if let Some(kb) = self.keyboard.take() {
            kb.release();
        }
        if let Some(ptr) = self.pointer.take() {
            ptr.release();
        }
        if let Some(seat) = self.seat.take() {
            seat.release();
        }
        for out in self.outputs.drain(..) {
            out.output.release();
        }
        self.output = None;
        if let Some(vp) = self.viewporter.take() {
            vp.destroy();
        }
        if let Some(ls) = self.layer_shell.take() {
            ls.destroy();
        }
        if let Some(wm) = self.xdg_wm_base.take() {
            wm.destroy();
        }
        let _ = event_queue.roundtrip(self);
    }
}
