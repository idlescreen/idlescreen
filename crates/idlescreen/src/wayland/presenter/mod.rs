// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Wayland fullscreen presentation engine supporting xdg_wm_base (GNOME Mutter),
//! zwlr_layer_shell_v1 (wlroots/COSMIC), wp_viewporter 1x cell-resolution scaling,
//! direct wl_shm ARGB8888 blitting, and input event dismissal.

mod daemon_run;
mod frame_step;
mod preview_run;
pub mod shm_buf;
pub mod wl_dispatch;
pub mod wl_state;

#[cfg(test)]
mod input_tests;

use std::time::{Duration, Instant};

use wayland_client::WEnum;
use wayland_client::protocol::{wl_keyboard, wl_output, wl_pointer, wl_surface};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

use crate::wayland::viewport::SurfaceViewport;
pub use daemon_run::run_wayland_daemon_presenter;
pub use frame_step::viewport_buffer_dimensions;
pub use preview_run::run_wayland_fullscreen_preview;
pub use shm_buf::{
    MappedBuffer, allocate_shm_buffer, allocate_shm_buffer_with_format, compute_cell_damage_rect,
};
pub use wl_state::WaylandState;

pub const INPUT_DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);
pub const POINTER_DRIFT_THRESHOLD_PX: f64 = 15.0;

#[derive(Clone, Debug)]
pub struct DetectedOutput {
    pub name: u32,
    pub connector_name: String,
    pub output: wl_output::WlOutput,
    pub width: u32,
    pub height: u32,
    pub refresh_mhz: i32,
}

pub struct PresentationSurface {
    pub output: Option<wl_output::WlOutput>,
    pub surface: wl_surface::WlSurface,
    pub viewport: SurfaceViewport,
    pub layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pub xdg_surface: Option<xdg_surface::XdgSurface>,
    pub xdg_toplevel: Option<xdg_toplevel::XdgToplevel>,
    pub configured: bool,
    pub configured_width: u32,
    pub configured_height: u32,
    pub width: u32,
    pub height: u32,
    pub refresh_mhz: i32,
    pub buffers: Option<[MappedBuffer; 2]>,
    pub current_buf: usize,
    pub displayed_cells: Vec<crate::cell::TerminalCell>,
    pub displayed_subcell_gen: u64,
    pub needs_commit: bool,
}

impl WaylandState {
    pub fn handle_key_event(&mut self, key_state: WEnum<wl_keyboard::KeyState>) {
        if matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed)) {
            self.handle_pointer_axis();
        }
    }

    pub fn handle_key_code(&mut self, key: u32, key_state: WEnum<wl_keyboard::KeyState>) {
        if matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed)) {
            if self.preview_mode {
                match key {
                    15 | 27 | 49 | 106 | 108 => {
                        self.scene_step_delta += 1;
                        return;
                    }
                    25 | 26 | 103 | 105 => {
                        self.scene_step_delta -= 1;
                        return;
                    }
                    _ => {}
                }
            }
            self.handle_pointer_axis();
        }
    }

    pub fn handle_pointer_button(&mut self, btn_state: WEnum<wl_pointer::ButtonState>) {
        if matches!(btn_state, WEnum::Value(wl_pointer::ButtonState::Pressed)) {
            self.handle_pointer_axis();
        }
    }

    pub fn request_exit(&mut self) {
        self.exit_requested = true;
        self.dpms.wake_outputs();
    }

    pub fn handle_pointer_motion(&mut self, surface_x: f64, surface_y: f64) {
        if self.start_time.elapsed() < INPUT_DEBOUNCE_WINDOW {
            self.initial_pointer_pos = Some((surface_x, surface_y));
        } else if let Some((ix, iy)) = self.initial_pointer_pos {
            if (surface_x - ix).abs().max((surface_y - iy).abs()) > POINTER_DRIFT_THRESHOLD_PX {
                self.request_exit();
            }
        } else {
            self.initial_pointer_pos = Some((surface_x, surface_y));
        }
    }

    pub fn handle_pointer_enter(&mut self, surface_x: f64, surface_y: f64) {
        if self.start_time.elapsed() < INPUT_DEBOUNCE_WINDOW || self.initial_pointer_pos.is_none() {
            self.initial_pointer_pos = Some((surface_x, surface_y));
        }
    }

    pub fn handle_pointer_axis(&mut self) {
        if self.start_time.elapsed() >= INPUT_DEBOUNCE_WINDOW {
            self.request_exit();
        }
    }
}

/// Computes chunky 16-bit SNES arcade grid dimensions (`120x36` on 16:9/16:10 HD/QHD/4K displays,
/// aspect-scaled on 21:9/32:9 ultrawide and portrait displays) so the pixel-art banner
/// commands the screen with crisp `~1:2` pixel blocks.
pub fn arcade_grid_dimensions(pixel_w: usize, pixel_h: usize) -> (usize, usize) {
    if pixel_w >= 960 && pixel_h >= 540 && pixel_w >= pixel_h {
        let rows = 36usize;
        let cols = if pixel_w * 9 > pixel_h * 18 {
            ((120 * pixel_w * 9) / (pixel_h * 16)).clamp(120, 256)
        } else {
            120
        };
        (cols, rows)
    } else if pixel_h > pixel_w && pixel_w >= 720 && pixel_h >= 960 {
        let cols = 96usize;
        let rows = ((96 * pixel_h * 9) / (pixel_w * 16)).clamp(48, 108);
        (cols, rows)
    } else {
        ((pixel_w / 8).clamp(10, 120), (pixel_h / 16).clamp(5, 36))
    }
}

/// Caches Linux sysfs battery discharge status to avoid per-frame filesystem reads.
pub struct PowerThrottle {
    last_check: Instant,
    on_battery: bool,
}

impl Default for PowerThrottle {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerThrottle {
    pub fn new() -> Self {
        let on_battery = probe_battery_discharging();
        Self {
            last_check: Instant::now(),
            on_battery,
        }
    }

    pub fn effective_frame_dur(&mut self, base_dur: Duration, is_settled: bool) -> Duration {
        if self.last_check.elapsed() >= Duration::from_secs(5) {
            self.on_battery = probe_battery_discharging();
            self.last_check = Instant::now();
        }
        if is_settled || self.on_battery {
            base_dur.max(Duration::from_nanos(1_000_000_000 / 30))
        } else {
            base_dur
        }
    }
}

fn probe_battery_discharging() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("BAT")
            && let Ok(status) = std::fs::read_to_string(entry.path().join("status"))
            && status.trim().eq_ignore_ascii_case("Discharging")
        {
            return true;
        }
    }
    false
}
