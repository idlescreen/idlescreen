// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Wayland fullscreen presentation engine supporting xdg_wm_base (GNOME Mutter)
//! and zwlr_layer_shell_v1 (wlroots/COSMIC), direct wl_shm ARGB8888 blitting,
//! and input event dismissal.

#![allow(clippy::all, clippy::pedantic)]

use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_registry, wl_seat, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::ascii::Ascii;
use crate::ascii::effects::EffectKind;
use crate::ascii::state::TextMode;
use crate::cell::TerminalCell;
use crate::extras_ipc::ExtrasSession;
use crate::wayland::blit::blit_cells_to_argb;

pub struct MappedBuffer {
    pub wl_buffer: wl_buffer::WlBuffer,
    _memfd: OwnedFd,
    mapped_ptr: *mut u32,
    pub width: u32,
    pub height: u32,
}

impl Drop for MappedBuffer {
    fn drop(&mut self) {
        self.wl_buffer.destroy();
        if !self.mapped_ptr.is_null() && self.width > 0 && self.height > 0 {
            let length = (self.width as usize) * (self.height as usize) * 4;
            unsafe {
                libc::munmap(self.mapped_ptr as *mut libc::c_void, length);
            }
        }
    }
}

impl MappedBuffer {
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        let len = (self.width as usize) * (self.height as usize);
        unsafe { std::slice::from_raw_parts_mut(self.mapped_ptr, len) }
    }
}

fn poll_and_read_events<D>(
    connection: &Connection,
    event_queue: &mut wayland_client::EventQueue<D>,
    state: &mut D,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    connection.flush()?;

    if let Some(guard) = connection.prepare_read() {
        let fd = guard.connection_fd().as_raw_fd();
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        let timeout_ms = timeout.as_millis().min(50) as libc::c_int;
        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if ret > 0 && (pfd.revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP)) != 0 {
            let _ = guard.read();
        }
    }

    event_queue.dispatch_pending(state)?;
    Ok(())
}

pub fn allocate_shm_buffer(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
    width: u32,
    height: u32,
) -> Option<MappedBuffer> {
    if width == 0 || height == 0 {
        return None;
    }
    let length = (width as usize) * (height as usize) * 4;
    let fd = unsafe { libc::memfd_create(c"idlescreen-shm".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return None;
    }
    let memfd = unsafe { OwnedFd::from_raw_fd(fd) };
    if unsafe { libc::ftruncate(memfd.as_fd().as_raw_fd(), length as i64) } != 0 {
        return None;
    }
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            length,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            memfd.as_fd().as_raw_fd(),
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return None;
    }

    let pool = shm.create_pool(memfd.as_fd(), length as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        (width * 4) as i32,
        wl_shm::Format::Argb8888,
        qh,
        (),
    );
    pool.destroy();

    Some(MappedBuffer {
        wl_buffer: buffer,
        _memfd: memfd,
        mapped_ptr: ptr as *mut u32,
        width,
        height,
    })
}

pub const INPUT_DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);
pub const POINTER_DRIFT_THRESHOLD_PX: f64 = 15.0;

#[derive(Clone, Debug)]
pub struct DetectedOutput {
    pub name: u32,
    pub output: wl_output::WlOutput,
    pub width: u32,
    pub height: u32,
    pub refresh_mhz: i32,
}

pub struct PresentationSurface {
    pub output: Option<wl_output::WlOutput>,
    pub surface: wl_surface::WlSurface,
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
}

pub struct WaylandState {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub shm: Option<wl_shm::WlShm>,
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
    pub surface: Option<wl_surface::WlSurface>,
    pub layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pub xdg_surface: Option<xdg_surface::XdgSurface>,
    pub xdg_toplevel: Option<xdg_toplevel::XdgToplevel>,
    pub configured: bool,
    pub configured_width: u32,
    pub configured_height: u32,
    pub exit_requested: bool,
    pub initial_pointer_pos: Option<(f64, f64)>,
    pub start_time: Instant,
    pub dpms: crate::wayland::dpms::DpmsController,
}

impl WaylandState {
    pub fn new() -> Self {
        Self {
            compositor: None,
            shm: None,
            seat: None,
            keyboard: None,
            pointer: None,
            layer_shell: None,
            xdg_wm_base: None,
            output: None,
            output_width: 0,
            output_height: 0,
            output_refresh_mhz: 0,
            outputs: Vec::new(),
            surfaces: Vec::new(),
            surface: None,
            layer_surface: None,
            xdg_surface: None,
            xdg_toplevel: None,
            configured: false,
            configured_width: 0,
            configured_height: 0,
            exit_requested: false,
            initial_pointer_pos: None,
            start_time: Instant::now(),
            dpms: crate::wayland::dpms::DpmsController::new(),
        }
    }

    pub fn all_configured(&self) -> bool {
        if self.surfaces.is_empty() {
            self.configured
        } else {
            self.surfaces.iter().all(|s| s.configured)
        }
    }

    pub fn any_configured(&self) -> bool {
        if self.surfaces.is_empty() {
            self.configured
        } else {
            self.surfaces.iter().any(|s| s.configured)
        }
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
            let mut layer_surface_opt = None;
            let mut xdg_surface_opt = None;
            let mut xdg_toplevel_opt = None;

            if let Some(layer_shell) = &self.layer_shell {
                let layer_surface = layer_shell.get_layer_surface(
                    &surface,
                    target_output.as_ref(),
                    zwlr_layer_shell_v1::Layer::Overlay,
                    "idlescreen".to_string(),
                    qh,
                    (),
                );
                layer_surface.set_anchor(
                    zwlr_layer_surface_v1::Anchor::Top
                        | zwlr_layer_surface_v1::Anchor::Bottom
                        | zwlr_layer_surface_v1::Anchor::Left
                        | zwlr_layer_surface_v1::Anchor::Right,
                );
                layer_surface.set_margin(0, 0, 0, 0);
                layer_surface.set_exclusive_zone(-1);
                layer_surface.set_keyboard_interactivity(
                    zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
                );
                layer_surface.set_size(0, 0);
                surface.commit();
                layer_surface_opt = Some(layer_surface);
            } else if let Some(xdg_wm_base) = &self.xdg_wm_base {
                let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, qh, ());
                let toplevel = xdg_surface.get_toplevel(qh, ());
                toplevel.set_title("idlescreen".to_string());
                toplevel.set_app_id("idlescreen".to_string());
                toplevel.set_fullscreen(target_output.as_ref());
                surface.commit();
                xdg_surface_opt = Some(xdg_surface);
                xdg_toplevel_opt = Some(toplevel);
            }

            let (out_w, out_h, refresh) = if let Some(ref o) = target_output {
                self.outputs
                    .iter()
                    .find(|out| &out.output == o)
                    .map_or((0, 0, 0), |out| (out.width, out.height, out.refresh_mhz))
            } else {
                (
                    self.output_width,
                    self.output_height,
                    self.output_refresh_mhz,
                )
            };

            let pres_surf = PresentationSurface {
                output: target_output,
                surface: surface.clone(),
                layer_surface: layer_surface_opt.clone(),
                xdg_surface: xdg_surface_opt.clone(),
                xdg_toplevel: xdg_toplevel_opt.clone(),
                configured: false,
                configured_width: 0,
                configured_height: 0,
                width: out_w,
                height: out_h,
                refresh_mhz: refresh,
                buffers: None,
                current_buf: 0,
            };

            if self.surface.is_none() {
                self.surface = Some(surface);
                self.layer_surface = layer_surface_opt;
                self.xdg_surface = xdg_surface_opt;
                self.xdg_toplevel = xdg_toplevel_opt;
            }
            self.surfaces.push(pres_surf);
        }

        Ok(())
    }

    pub fn cleanup_presentation(&mut self, event_queue: &mut wayland_client::EventQueue<Self>) {
        for mut s in self.surfaces.drain(..) {
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
        if let Some(layer) = self.layer_surface.take() {
            layer.destroy();
        }
        if let Some(toplevel) = self.xdg_toplevel.take() {
            toplevel.destroy();
        }
        if let Some(xdg_surface) = self.xdg_surface.take() {
            xdg_surface.destroy();
        }
        if let Some(surf) = self.surface.take() {
            surf.destroy();
        }
        let _ = event_queue.roundtrip(self);
    }

    pub fn handle_key_event(&mut self, key_state: WEnum<wl_keyboard::KeyState>) {
        if matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed))
            && self.start_time.elapsed() >= INPUT_DEBOUNCE_WINDOW
        {
            self.exit_requested = true;
        }
    }

    pub fn handle_pointer_button(&mut self, btn_state: WEnum<wl_pointer::ButtonState>) {
        if matches!(btn_state, WEnum::Value(wl_pointer::ButtonState::Pressed))
            && self.start_time.elapsed() >= INPUT_DEBOUNCE_WINDOW
        {
            self.exit_requested = true;
        }
    }

    pub fn handle_pointer_motion(&mut self, surface_x: f64, surface_y: f64) {
        if self.start_time.elapsed() < INPUT_DEBOUNCE_WINDOW {
            self.initial_pointer_pos = Some((surface_x, surface_y));
        } else if let Some((init_x, init_y)) = self.initial_pointer_pos {
            let dx = (surface_x - init_x).abs();
            let dy = (surface_y - init_y).abs();
            if dx > POINTER_DRIFT_THRESHOLD_PX || dy > POINTER_DRIFT_THRESHOLD_PX {
                self.exit_requested = true;
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
            self.exit_requested = true;
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(registry.bind(name, version.min(4), qh, ()));
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base = Some(registry.bind(name, version.min(2), qh, ()));
                }
                "wl_output" => {
                    let output: wl_output::WlOutput = registry.bind(name, version.min(4), qh, ());
                    if state.output.is_none() {
                        state.output = Some(output.clone());
                    }
                    state.outputs.push(DetectedOutput {
                        name,
                        output,
                        width: 0,
                        height: 0,
                        refresh_mhz: 0,
                    });
                }
                "wl_seat" if state.seat.is_none() => {
                    let seat: wl_seat::WlSeat = registry.bind(name, version.min(7), qh, ());
                    state.pointer = Some(seat.get_pointer(qh, ()));
                    state.keyboard = Some(seat.get_keyboard(qh, ()));
                    state.seat = Some(seat);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Mode {
            flags,
            width,
            height,
            refresh,
            ..
        } = event
        {
            let is_current = match flags {
                WEnum::Value(wl_output::Mode::Current) => true,
                WEnum::Unknown(val) => (val & 1) != 0,
                _ => false,
            };
            if is_current && width > 0 && height > 0 {
                if let Some(out) = state.outputs.iter_mut().find(|o| &o.output == output) {
                    out.width = width as u32;
                    out.height = height as u32;
                    out.refresh_mhz = refresh;
                }
                if state.output.as_ref() == Some(output) || state.output_width == 0 {
                    state.output_width = width as u32;
                    state.output_height = height as u32;
                    state.output_refresh_mhz = refresh;
                }
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandState {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for WaylandState {
    fn event(
        state: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            if let Some(surf) = state
                .surfaces
                .iter_mut()
                .find(|s| s.xdg_surface.as_ref() == Some(surface))
            {
                surf.configured = true;
            }
            if state.xdg_surface.as_ref() == Some(surface) || !state.configured {
                state.configured = true;
            }
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for WaylandState {
    fn event(
        state: &mut Self,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                if width > 0 && height > 0 {
                    if let Some(surf) = state
                        .surfaces
                        .iter_mut()
                        .find(|s| s.xdg_toplevel.as_ref() == Some(toplevel))
                    {
                        surf.configured_width = width as u32;
                        surf.configured_height = height as u32;
                    }
                    if state.xdg_toplevel.as_ref() == Some(toplevel) || state.configured_width == 0
                    {
                        state.configured_width = width as u32;
                        state.configured_height = height as u32;
                    }
                }
            }
            xdg_toplevel::Event::Close => {
                state.exit_requested = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                layer_surface.ack_configure(serial);
                if let Some(surf) = state
                    .surfaces
                    .iter_mut()
                    .find(|s| s.layer_surface.as_ref() == Some(layer_surface))
                {
                    surf.configured = true;
                    if width > 0 && height > 0 {
                        surf.configured_width = width;
                        surf.configured_height = height;
                    }
                }
                if state.layer_surface.as_ref() == Some(layer_surface) || !state.configured {
                    state.configured = true;
                    if width > 0 && height > 0 {
                        state.configured_width = width;
                        state.configured_height = height;
                    }
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.exit_requested = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            state: key_state, ..
        } = event
        {
            state.handle_key_event(key_state);
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            } => {
                state.handle_pointer_enter(surface_x, surface_y);
            }
            wl_pointer::Event::Button {
                state: btn_state, ..
            } => {
                state.handle_pointer_button(btn_state);
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.handle_pointer_motion(surface_x, surface_y);
            }
            wl_pointer::Event::Axis { .. } => {
                state.handle_pointer_axis();
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// Runs a borderless fullscreen screensaver preview on Wayland.
pub fn run_wayland_fullscreen_preview(
    saver_name: &str,
    mode_str: &str,
    dwell_secs: f32,
    custom_text: &str,
    timeout: Option<Duration>,
    config_fps: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();
    let _registry = connection.display().get_registry(&qh, ());

    let mut state = WaylandState::new();
    event_queue.roundtrip(&mut state)?;

    state.create_presentation_surfaces(&qh)?;

    let configure_start = Instant::now();
    let configure_timeout = Duration::from_millis(500);
    while !state.all_configured() && !state.exit_requested {
        if configure_start.elapsed() >= configure_timeout {
            if !state.any_configured() {
                return Err("Wayland surface configure negotiation timed out".into());
            }
            break;
        }
        poll_and_read_events(
            &connection,
            &mut event_queue,
            &mut state,
            Duration::from_millis(50),
        )?;
    }

    if state.exit_requested {
        state.cleanup_presentation(&mut event_queue);
        return Ok(());
    }

    let shm = state.shm.clone().ok_or("wl_shm not available")?;
    for s in &mut state.surfaces {
        let width = if s.configured_width > 0 {
            s.configured_width
        } else if s.width > 0 {
            s.width
        } else {
            1920
        };
        let height = if s.configured_height > 0 {
            s.configured_height
        } else if s.height > 0 {
            s.height
        } else {
            1080
        };
        s.configured_width = width;
        s.configured_height = height;
        s.width = width;
        s.height = height;
        let buf0 = allocate_shm_buffer(&shm, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 0")?;
        let buf1 = allocate_shm_buffer(&shm, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 1")?;
        s.buffers = Some([buf0, buf1]);
    }

    let primary_width = state.surfaces.first().map_or(1920, |s| s.configured_width);
    let primary_height = state.surfaces.first().map_or(1080, |s| s.configured_height);
    let cols = (primary_width as usize / 8).max(10);
    let rows = (primary_height as usize / 16).max(5);

    let effect_kind = EffectKind::parse(saver_name);
    let is_superfedora = saver_name == "superfedora";
    let is_superfedora_unlocked =
        is_superfedora && crate::ascii::sys_info::is_superfedora_machine();
    let is_extras_cand = effect_kind.is_none() && saver_name != "ascii" && !is_superfedora;
    let mut is_extras = is_extras_cand && ExtrasSession::is_available();
    let mut extras_session = if is_extras {
        ExtrasSession::spawn(saver_name, cols, rows).ok()
    } else {
        None
    };
    if extras_session.is_none() {
        is_extras = false;
    }

    let target_fps = if config_fps > 0 {
        config_fps
    } else if state.output_refresh_mhz > 0 {
        ((state.output_refresh_mhz + 500) / 1000).max(1) as u32
    } else {
        60
    };
    let frame_dur = Duration::from_nanos(1_000_000_000 / target_fps as u64);

    let text_mode = TextMode::parse(mode_str);
    let custom_opt = (!custom_text.trim().is_empty()).then(|| custom_text.to_string());
    let targets = crate::daemon::config::parse_targets_from_mode(mode_str);
    let mut ascii_saver = if is_superfedora_unlocked {
        Ascii::new_superfedora()
            .with_dwell(dwell_secs)
            .with_fps(target_fps)
    } else {
        let mut saver = Ascii::new()
            .with_mode(text_mode, custom_opt)
            .with_targets(targets)
            .with_dwell(dwell_secs)
            .with_fps(target_fps);
        if let Some(effect) = effect_kind {
            saver.pin_effect(effect);
        }
        saver
    };
    ascii_saver.init(cols, rows);

    let mut cells = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = Instant::now();
    state.start_time = Instant::now();

    while !state.exit_requested {
        if let Some(limit) = timeout {
            if state.start_time.elapsed() >= limit {
                break;
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        if dt >= frame_dur {
            last_frame = now;

            let max_cols = state
                .surfaces
                .iter()
                .map(|s| (s.configured_width as usize / 8).max(10))
                .max()
                .unwrap_or(cols);
            let max_rows = state
                .surfaces
                .iter()
                .map(|s| (s.configured_height as usize / 16).max(5))
                .max()
                .unwrap_or(rows);

            if is_extras {
                if let Some(ref mut s) = extras_session {
                    if s.tick_and_draw(dt, &mut cells, max_cols, max_rows).is_err() {
                        is_extras = false;
                        ascii_saver.init(max_cols, max_rows);
                    }
                }
            }
            if !is_extras {
                ascii_saver.update(dt, max_cols, max_rows);
            }

            for s in &mut state.surfaces {
                let cur_w = if s.configured_width > 0 {
                    s.configured_width
                } else {
                    1920
                };
                let cur_h = if s.configured_height > 0 {
                    s.configured_height
                } else {
                    1080
                };

                if let Some(ref mut bufs) = s.buffers {
                    if cur_w != bufs[s.current_buf].width || cur_h != bufs[s.current_buf].height {
                        if let Some(new_b) = allocate_shm_buffer(&shm, &qh, cur_w, cur_h) {
                            bufs[s.current_buf] = new_b;
                        }
                    }

                    let buf = &mut bufs[s.current_buf];
                    let buf_w = buf.width as usize;
                    let buf_h = buf.height as usize;
                    let out_cols = (buf_w / 8).max(10);
                    let out_rows = (buf_h / 16).max(5);

                    if cells.len() < out_cols * out_rows {
                        cells.resize(out_cols * out_rows, TerminalCell::default());
                    }

                    if !is_extras {
                        ascii_saver.draw(&mut cells[..out_cols * out_rows], out_cols, out_rows);
                    }

                    blit_cells_to_argb(
                        &cells[..out_cols * out_rows],
                        out_cols,
                        out_rows,
                        buf.as_mut_slice(),
                        buf_w,
                        buf_h,
                    );

                    s.surface.attach(Some(&buf.wl_buffer), 0, 0);
                    s.surface.damage_buffer(0, 0, buf_w as i32, buf_h as i32);
                    s.surface.commit();
                    s.current_buf ^= 1;
                }
            }

            connection.flush()?;
        }

        let elapsed = last_frame.elapsed();
        let sleep_dur = frame_dur.saturating_sub(elapsed);
        poll_and_read_events(&connection, &mut event_queue, &mut state, sleep_dur)?;
    }

    state.cleanup_presentation(&mut event_queue);
    Ok(())
}

/// Runs fullscreen Wayland screensaver presentation for the background daemon while active.
pub fn run_wayland_daemon_presenter(
    state: Arc<crate::daemon::dbus::ScreenSaverState>,
    dwell_secs: f32,
    mode_str: &str,
    custom_text: &str,
    config_fps: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue();
    let qh = event_queue.handle();
    let _registry = connection.display().get_registry(&qh, ());

    let mut state_wl = WaylandState::new();
    event_queue.roundtrip(&mut state_wl)?;

    state_wl.create_presentation_surfaces(&qh)?;

    let configure_start = Instant::now();
    let configure_timeout = Duration::from_millis(500);
    while !state_wl.all_configured()
        && !state_wl.exit_requested
        && state.active.load(Ordering::SeqCst)
    {
        if configure_start.elapsed() >= configure_timeout {
            if !state_wl.any_configured() {
                return Err("Wayland surface configure negotiation timed out".into());
            }
            break;
        }
        poll_and_read_events(
            &connection,
            &mut event_queue,
            &mut state_wl,
            Duration::from_millis(50),
        )?;
    }

    if state_wl.exit_requested || !state.active.load(Ordering::SeqCst) {
        if state_wl.exit_requested {
            state.stop();
        }
        state_wl.cleanup_presentation(&mut event_queue);
        return Ok(());
    }

    let shm = state_wl.shm.clone().ok_or("wl_shm not available")?;
    for s in &mut state_wl.surfaces {
        let width = if s.configured_width > 0 {
            s.configured_width
        } else if s.width > 0 {
            s.width
        } else {
            1920
        };
        let height = if s.configured_height > 0 {
            s.configured_height
        } else if s.height > 0 {
            s.height
        } else {
            1080
        };
        s.configured_width = width;
        s.configured_height = height;
        s.width = width;
        s.height = height;
        let buf0 = allocate_shm_buffer(&shm, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 0")?;
        let buf1 = allocate_shm_buffer(&shm, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 1")?;
        s.buffers = Some([buf0, buf1]);
    }

    let primary_width = state_wl
        .surfaces
        .first()
        .map_or(1920, |s| s.configured_width);
    let primary_height = state_wl
        .surfaces
        .first()
        .map_or(1080, |s| s.configured_height);
    let cols = (primary_width as usize / 8).max(10);
    let rows = (primary_height as usize / 16).max(5);

    let saver_name = state
        .current_saver
        .lock()
        .map_or_else(|_| "ascii".to_string(), |s| s.clone());
    let effect_kind = EffectKind::parse(&saver_name);
    let is_superfedora = saver_name == "superfedora";
    let is_superfedora_unlocked =
        is_superfedora && crate::ascii::sys_info::is_superfedora_machine();
    let is_extras_cand = effect_kind.is_none() && saver_name != "ascii" && !is_superfedora;
    let mut is_extras = is_extras_cand && ExtrasSession::is_available();
    let mut extras_session = if is_extras {
        ExtrasSession::spawn(&saver_name, cols, rows).ok()
    } else {
        None
    };
    if extras_session.is_none() {
        is_extras = false;
    }

    let target_fps = if config_fps > 0 {
        config_fps
    } else if state_wl.output_refresh_mhz > 0 {
        ((state_wl.output_refresh_mhz + 500) / 1000).max(1) as u32
    } else {
        60
    };
    let frame_dur = Duration::from_nanos(1_000_000_000 / target_fps as u64);

    let text_mode = TextMode::parse(mode_str);
    let custom_opt = if let Ok(guard) = state.custom_text.lock() {
        (!guard.trim().is_empty()).then(|| guard.clone())
    } else {
        (!custom_text.trim().is_empty()).then(|| custom_text.to_string())
    };
    let targets = if let Ok(guard) = state.text_targets.lock()
        && !guard.is_empty()
    {
        guard.clone()
    } else {
        crate::daemon::config::parse_targets_from_mode(mode_str)
    };
    let mut ascii_saver = if is_superfedora_unlocked {
        Ascii::new_superfedora()
            .with_dwell(dwell_secs)
            .with_fps(target_fps)
    } else {
        let mut saver = Ascii::new()
            .with_mode(text_mode, custom_opt)
            .with_targets(targets)
            .with_dwell(dwell_secs)
            .with_fps(target_fps);
        if let Some(effect) = effect_kind {
            saver.pin_effect(effect);
        }
        saver
    };
    ascii_saver.init(cols, rows);

    let mut cells = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = Instant::now();
    let mut last_custom_epoch = state.custom_text_epoch.load(Ordering::Acquire);
    state_wl.start_time = Instant::now();

    while state.active.load(Ordering::SeqCst) && !state_wl.exit_requested {
        // Sync live target and custom text changes from D-Bus / socket
        if let Ok(guard) = state.text_targets.try_lock()
            && *guard != ascii_saver.rotation.targets
            && !guard.is_empty()
        {
            ascii_saver.rotation.set_targets(guard.clone());
        }
        let cur_epoch = state.custom_text_epoch.load(Ordering::Acquire);
        if cur_epoch != last_custom_epoch {
            if let Ok(guard) = state.custom_text.lock() {
                let opt = (!guard.trim().is_empty()).then(|| guard.clone());
                if opt != ascii_saver.rotation.custom_text {
                    ascii_saver.rotation.custom_text = opt;
                    if !is_extras {
                        ascii_saver.init(cols, rows);
                    }
                }
                last_custom_epoch = cur_epoch;
            }
        }

        // Park rendering immediately when output power is off (DPMS)
        if state_wl.dpms.is_sleeping() {
            poll_and_read_events(
                &connection,
                &mut event_queue,
                &mut state_wl,
                Duration::from_millis(50),
            )?;
            continue;
        }

        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        if dt >= frame_dur {
            last_frame = now;

            let max_cols = state_wl
                .surfaces
                .iter()
                .map(|s| (s.configured_width as usize / 8).max(10))
                .max()
                .unwrap_or(cols);
            let max_rows = state_wl
                .surfaces
                .iter()
                .map(|s| (s.configured_height as usize / 16).max(5))
                .max()
                .unwrap_or(rows);

            if is_extras {
                if let Some(ref mut s) = extras_session {
                    if s.tick_and_draw(dt, &mut cells, max_cols, max_rows).is_err() {
                        is_extras = false;
                        ascii_saver.init(max_cols, max_rows);
                    }
                }
            }
            if !is_extras {
                ascii_saver.update(dt, max_cols, max_rows);
            }

            for s in &mut state_wl.surfaces {
                let cur_w = if s.configured_width > 0 {
                    s.configured_width
                } else {
                    1920
                };
                let cur_h = if s.configured_height > 0 {
                    s.configured_height
                } else {
                    1080
                };

                if let Some(ref mut bufs) = s.buffers {
                    if cur_w != bufs[s.current_buf].width || cur_h != bufs[s.current_buf].height {
                        if let Some(new_b) = allocate_shm_buffer(&shm, &qh, cur_w, cur_h) {
                            bufs[s.current_buf] = new_b;
                        }
                    }

                    let buf = &mut bufs[s.current_buf];
                    let buf_w = buf.width as usize;
                    let buf_h = buf.height as usize;
                    let out_cols = (buf_w / 8).max(10);
                    let out_rows = (buf_h / 16).max(5);

                    if cells.len() < out_cols * out_rows {
                        cells.resize(out_cols * out_rows, TerminalCell::default());
                    }

                    if !is_extras {
                        ascii_saver.draw(&mut cells[..out_cols * out_rows], out_cols, out_rows);
                    }

                    blit_cells_to_argb(
                        &cells[..out_cols * out_rows],
                        out_cols,
                        out_rows,
                        buf.as_mut_slice(),
                        buf_w,
                        buf_h,
                    );

                    s.surface.attach(Some(&buf.wl_buffer), 0, 0);
                    s.surface.damage_buffer(0, 0, buf_w as i32, buf_h as i32);
                    s.surface.commit();
                    s.current_buf ^= 1;
                }
            }

            connection.flush()?;

            if !is_extras && ascii_saver.is_settled() {
                // Zero-FPS settled dwell: commit settled frame once, then park presenter thread
                while ascii_saver.is_settled()
                    && ascii_saver.dwell_left > 0.0
                    && state.active.load(Ordering::SeqCst)
                    && !state_wl.exit_requested
                    && !state_wl.dpms.is_sleeping()
                {
                    let slice = Duration::from_secs_f32(ascii_saver.dwell_left.min(0.05));
                    let slice_start = Instant::now();
                    poll_and_read_events(&connection, &mut event_queue, &mut state_wl, slice)?;
                    let slice_elapsed = slice_start.elapsed();
                    ascii_saver.dwell_left =
                        (ascii_saver.dwell_left - slice_elapsed.as_secs_f32()).max(0.0);

                    if state.custom_text_epoch.load(Ordering::Acquire) != last_custom_epoch {
                        ascii_saver.dwell_left = 0.0;
                        break;
                    }
                }
                last_frame = Instant::now() - frame_dur;
                continue;
            }
        }

        let elapsed = last_frame.elapsed();
        let sleep_dur = frame_dur.saturating_sub(elapsed);
        poll_and_read_events(&connection, &mut event_queue, &mut state_wl, sleep_dur)?;
    }

    if state_wl.exit_requested {
        state.stop();
    }

    state_wl.cleanup_presentation(&mut event_queue);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_debounce_constants() {
        assert_eq!(INPUT_DEBOUNCE_WINDOW, Duration::from_millis(500));
        assert_eq!(POINTER_DRIFT_THRESHOLD_PX, 15.0);
    }

    #[test]
    fn test_key_debounce_ignores_release_and_early_press() {
        let mut state = WaylandState::new();

        // Release event should always be ignored
        state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Released));
        assert!(!state.exit_requested);

        // Early press during the 500ms debounce window should be ignored
        state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Pressed));
        assert!(!state.exit_requested);

        // Release event after debounce window should also be ignored
        state.start_time = Instant::now() - Duration::from_millis(600);
        state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Released));
        assert!(!state.exit_requested);

        // Press event after debounce window triggers exit
        state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Pressed));
        assert!(state.exit_requested);
    }

    #[test]
    fn test_pointer_button_debounce() {
        let mut state = WaylandState::new();

        // Release event ignored
        state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Released));
        assert!(!state.exit_requested);

        // Early press during debounce window ignored
        state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Pressed));
        assert!(!state.exit_requested);

        // Release after debounce window ignored
        state.start_time = Instant::now() - Duration::from_millis(600);
        state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Released));
        assert!(!state.exit_requested);

        // Press after debounce window triggers exit
        state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Pressed));
        assert!(state.exit_requested);
    }

    #[test]
    fn test_pointer_motion_settling_and_drift() {
        let mut state = WaylandState::new();

        // Motion during 500ms startup grace period continually updates initial_pointer_pos
        state.handle_pointer_motion(100.0, 100.0);
        assert_eq!(state.initial_pointer_pos, Some((100.0, 100.0)));
        assert!(!state.exit_requested);

        state.handle_pointer_motion(120.0, 120.0);
        assert_eq!(state.initial_pointer_pos, Some((120.0, 120.0)));
        assert!(!state.exit_requested);

        // After 500ms debounce window has elapsed:
        state.start_time = Instant::now() - Duration::from_millis(600);

        // Small drift <= 15px relative to settled position does NOT trigger exit
        state.handle_pointer_motion(125.0, 128.0);
        assert!(!state.exit_requested);

        // Intentional motion > 15px triggers exit
        state.handle_pointer_motion(140.0, 120.0);
        assert!(state.exit_requested);
    }

    #[test]
    fn test_pointer_enter_settles_position() {
        let mut state = WaylandState::new();

        state.handle_pointer_enter(50.0, 50.0);
        assert_eq!(state.initial_pointer_pos, Some((50.0, 50.0)));

        state.handle_pointer_enter(60.0, 60.0);
        assert_eq!(state.initial_pointer_pos, Some((60.0, 60.0)));
    }

    #[test]
    fn test_pointer_axis_debounce() {
        let mut state = WaylandState::new();

        // Early scroll during debounce window ignored
        state.handle_pointer_axis();
        assert!(!state.exit_requested);

        // Scroll after debounce window triggers exit
        state.start_time = Instant::now() - Duration::from_millis(600);
        state.handle_pointer_axis();
        assert!(state.exit_requested);
    }

    #[test]
    fn test_pointer_motion_without_prior_enter() {
        let mut state = WaylandState::new();
        state.start_time = Instant::now() - Duration::from_millis(600);
        assert_eq!(state.initial_pointer_pos, None);

        // First motion event after debounce establishes baseline position without exiting
        state.handle_pointer_motion(200.0, 300.0);
        assert_eq!(state.initial_pointer_pos, Some((200.0, 300.0)));
        assert!(!state.exit_requested);

        // Subsequent small movement within 15px threshold does not exit
        state.handle_pointer_motion(205.0, 305.0);
        assert!(!state.exit_requested);

        // Subsequent movement > 15px triggers exit
        state.handle_pointer_motion(220.0, 300.0);
        assert!(state.exit_requested);
    }

    #[test]
    fn test_unknown_key_and_button_events_ignored() {
        let mut state = WaylandState::new();
        state.start_time = Instant::now() - Duration::from_millis(600);

        state.handle_key_event(WEnum::Unknown(99));
        assert!(!state.exit_requested);

        state.handle_pointer_button(WEnum::Unknown(99));
        assert!(!state.exit_requested);
    }

    #[test]
    fn test_pointer_enter_after_debounce_establishes_position() {
        let mut state = WaylandState::new();
        state.start_time = Instant::now() - Duration::from_millis(600);

        // If no prior position, enter establishes position without exit
        state.handle_pointer_enter(50.0, 50.0);
        assert_eq!(state.initial_pointer_pos, Some((50.0, 50.0)));
        assert!(!state.exit_requested);

        // If position already established, enter after debounce window does not overwrite
        state.handle_pointer_enter(100.0, 100.0);
        assert_eq!(state.initial_pointer_pos, Some((50.0, 50.0)));
        assert!(!state.exit_requested);
    }

    #[test]
    fn test_subpixel_motion_and_negative_coords() {
        let mut state = WaylandState::new();
        state.handle_pointer_motion(-50.5, -30.25);
        assert_eq!(state.initial_pointer_pos, Some((-50.5, -30.25)));

        state.start_time = Instant::now() - Duration::from_millis(600);

        // Motion within 15px
        state.handle_pointer_motion(-40.0, -25.0);
        assert!(!state.exit_requested);

        // Motion > 15px in y axis
        state.handle_pointer_motion(-40.0, -10.0);
        assert!(state.exit_requested);
    }
}
