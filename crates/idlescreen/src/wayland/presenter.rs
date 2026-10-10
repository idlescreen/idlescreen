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
                "wl_output" if state.output.is_none() => {
                    state.output = Some(registry.bind(name, version.min(4), qh, ()));
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
        _: &wl_output::WlOutput,
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
                state.output_width = width as u32;
                state.output_height = height as u32;
                state.output_refresh_mhz = refresh;
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
            state.configured = true;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                if width > 0 && height > 0 {
                    state.configured_width = width as u32;
                    state.configured_height = height as u32;
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
                state.configured = true;
                if width > 0 && height > 0 {
                    state.configured_width = width;
                    state.configured_height = height;
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
        if let wl_keyboard::Event::Key { .. } = event {
            state.exit_requested = true;
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
                if state.initial_pointer_pos.is_none() {
                    state.initial_pointer_pos = Some((surface_x, surface_y));
                }
            }
            wl_pointer::Event::Button { .. } => {
                state.exit_requested = true;
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                if let Some((init_x, init_y)) = state.initial_pointer_pos {
                    if state.start_time.elapsed() > Duration::from_millis(300) {
                        let dx = (surface_x - init_x).abs();
                        let dy = (surface_y - init_y).abs();
                        if dx > 15.0 || dy > 15.0 {
                            state.exit_requested = true;
                        }
                    }
                } else {
                    state.initial_pointer_pos = Some((surface_x, surface_y));
                }
            }
            wl_pointer::Event::Axis { .. } => {
                if state.start_time.elapsed() > Duration::from_millis(300) {
                    state.exit_requested = true;
                }
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

    let compositor = state
        .compositor
        .as_ref()
        .ok_or("Wayland compositor not available")?;
    let _shm = state.shm.as_ref().ok_or("wl_shm not available")?;

    let surface = compositor.create_surface(&qh, ());
    state.surface = Some(surface.clone());

    if let Some(layer_shell) = &state.layer_shell {
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            state.output.as_ref(),
            zwlr_layer_shell_v1::Layer::Overlay,
            "idlescreen".to_string(),
            &qh,
            (),
        );
        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer_surface.set_exclusive_zone(-1);
        layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand);
        layer_surface.set_size(0, 0);
        surface.commit();
        state.layer_surface = Some(layer_surface);
    } else if let Some(xdg_wm_base) = &state.xdg_wm_base {
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surface.get_toplevel(&qh, ());
        toplevel.set_title("idlescreen".to_string());
        toplevel.set_app_id("idlescreen".to_string());
        toplevel.set_fullscreen(state.output.as_ref());
        surface.commit();
        state.xdg_surface = Some(xdg_surface);
        state.xdg_toplevel = Some(toplevel);
    } else {
        return Err("No layer shell or xdg_wm_base available".into());
    }

    let configure_start = Instant::now();
    let configure_timeout = Duration::from_millis(500);
    while !state.configured {
        if configure_start.elapsed() >= configure_timeout {
            return Err("Wayland surface configure negotiation timed out".into());
        }
        poll_and_read_events(
            &connection,
            &mut event_queue,
            &mut state,
            Duration::from_millis(50),
        )?;
    }

    let width = if state.configured_width > 0 {
        state.configured_width
    } else if state.output_width > 0 {
        state.output_width
    } else {
        1920
    };
    let height = if state.configured_height > 0 {
        state.configured_height
    } else if state.output_height > 0 {
        state.output_height
    } else {
        1080
    };

    let shm_ref = state.shm.as_ref().unwrap();
    let mut buffers = [
        allocate_shm_buffer(shm_ref, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 0")?,
        allocate_shm_buffer(shm_ref, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 1")?,
    ];
    let mut current_buf = 0;

    let effect_kind = EffectKind::parse(saver_name);
    let is_extras_cand = effect_kind.is_none() && saver_name != "ascii";
    let mut is_extras = is_extras_cand && ExtrasSession::is_available();
    let cols = (width as usize / 8).max(10);
    let rows = (height as usize / 16).max(5);
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
    let mut ascii_saver = Ascii::new()
        .with_mode(text_mode, custom_opt)
        .with_dwell(dwell_secs)
        .with_fps(target_fps);
    if let Some(effect) = effect_kind {
        ascii_saver.pin_effect(effect);
    }
    ascii_saver.init(cols, rows);

    let mut cells = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = Instant::now();

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

            let cur_w = if state.configured_width > 0 {
                state.configured_width
            } else {
                width
            };
            let cur_h = if state.configured_height > 0 {
                state.configured_height
            } else {
                height
            };

            if cur_w != buffers[current_buf].width || cur_h != buffers[current_buf].height {
                if let Some(new_b) =
                    allocate_shm_buffer(state.shm.as_ref().unwrap(), &qh, cur_w, cur_h)
                {
                    buffers[current_buf] = new_b;
                }
            }

            let buf = &mut buffers[current_buf];
            let buf_w = buf.width as usize;
            let buf_h = buf.height as usize;
            let cur_cols = (buf_w / 8).max(10);
            let cur_rows = (buf_h / 16).max(5);
            if cur_cols * cur_rows != cells.len() {
                cells.resize(cur_cols * cur_rows, TerminalCell::default());
            }

            if is_extras {
                if let Some(ref mut s) = extras_session {
                    if s.tick_and_draw(dt, &mut cells, cur_cols, cur_rows).is_err() {
                        is_extras = false;
                        ascii_saver.init(cur_cols, cur_rows);
                    }
                }
            }
            if !is_extras {
                ascii_saver.update(dt, cur_cols, cur_rows);
                ascii_saver.draw(&mut cells, cur_cols, cur_rows);
            }

            blit_cells_to_argb(&cells, cur_cols, cur_rows, buf.as_mut_slice(), buf_w, buf_h);

            if let Some(surf) = &state.surface {
                surf.attach(Some(&buf.wl_buffer), 0, 0);
                surf.damage_buffer(0, 0, buf_w as i32, buf_h as i32);
                surf.commit();
            }
            connection.flush()?;

            current_buf ^= 1;
        }

        let elapsed = last_frame.elapsed();
        let sleep_dur = frame_dur.saturating_sub(elapsed);
        poll_and_read_events(&connection, &mut event_queue, &mut state, sleep_dur)?;
    }

    // Cleanup resources
    if let Some(layer) = state.layer_surface.take() {
        layer.destroy();
    }
    if let Some(toplevel) = state.xdg_toplevel.take() {
        toplevel.destroy();
    }
    if let Some(xdg_surface) = state.xdg_surface.take() {
        xdg_surface.destroy();
    }
    if let Some(surf) = state.surface.take() {
        surf.destroy();
    }
    drop(buffers);
    let _ = event_queue.roundtrip(&mut state);

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

    let compositor = state_wl
        .compositor
        .as_ref()
        .ok_or("Wayland compositor not available")?;
    let _shm = state_wl.shm.as_ref().ok_or("wl_shm not available")?;

    let surface = compositor.create_surface(&qh, ());
    state_wl.surface = Some(surface.clone());

    if let Some(layer_shell) = &state_wl.layer_shell {
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            state_wl.output.as_ref(),
            zwlr_layer_shell_v1::Layer::Overlay,
            "idlescreen".to_string(),
            &qh,
            (),
        );
        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer_surface.set_exclusive_zone(-1);
        layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand);
        layer_surface.set_size(0, 0);
        surface.commit();
        state_wl.layer_surface = Some(layer_surface);
    } else if let Some(xdg_wm_base) = &state_wl.xdg_wm_base {
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surface.get_toplevel(&qh, ());
        toplevel.set_title("idlescreen".to_string());
        toplevel.set_app_id("idlescreen".to_string());
        toplevel.set_fullscreen(state_wl.output.as_ref());
        surface.commit();
        state_wl.xdg_surface = Some(xdg_surface);
        state_wl.xdg_toplevel = Some(toplevel);
    } else {
        return Err("No layer shell or xdg_wm_base available".into());
    }

    event_queue.roundtrip(&mut state_wl)?;

    let width = if state_wl.configured_width > 0 {
        state_wl.configured_width
    } else if state_wl.output_width > 0 {
        state_wl.output_width
    } else {
        1920
    };
    let height = if state_wl.configured_height > 0 {
        state_wl.configured_height
    } else if state_wl.output_height > 0 {
        state_wl.output_height
    } else {
        1080
    };

    let shm_ref = state_wl.shm.as_ref().unwrap();
    let mut buffers = [
        allocate_shm_buffer(shm_ref, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 0")?,
        allocate_shm_buffer(shm_ref, &qh, width, height)
            .ok_or("Failed to allocate wl_shm buffer 1")?,
    ];
    let mut current_buf = 0;

    let saver_name = state
        .current_saver
        .lock()
        .map_or_else(|_| "ascii".to_string(), |s| s.clone());
    let effect_kind = EffectKind::parse(&saver_name);
    let is_extras_cand = effect_kind.is_none() && saver_name != "ascii";
    let mut is_extras = is_extras_cand && ExtrasSession::is_available();
    let cols = (width as usize / 8).max(10);
    let rows = (height as usize / 16).max(5);
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
    let custom_opt = (!custom_text.trim().is_empty()).then(|| custom_text.to_string());
    let mut ascii_saver = Ascii::new()
        .with_mode(text_mode, custom_opt)
        .with_dwell(dwell_secs)
        .with_fps(target_fps);
    if let Some(effect) = effect_kind {
        ascii_saver.pin_effect(effect);
    }
    ascii_saver.init(cols, rows);

    let mut cells = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = Instant::now();

    while state.active.load(Ordering::SeqCst) && !state_wl.exit_requested {
        let now = Instant::now();
        let dt = now.duration_since(last_frame);
        if dt >= frame_dur {
            last_frame = now;

            let cur_w = if state_wl.configured_width > 0 {
                state_wl.configured_width
            } else {
                width
            };
            let cur_h = if state_wl.configured_height > 0 {
                state_wl.configured_height
            } else {
                height
            };

            if cur_w != buffers[current_buf].width || cur_h != buffers[current_buf].height {
                if let Some(new_b) =
                    allocate_shm_buffer(state_wl.shm.as_ref().unwrap(), &qh, cur_w, cur_h)
                {
                    buffers[current_buf] = new_b;
                }
            }

            let buf = &mut buffers[current_buf];
            let buf_w = buf.width as usize;
            let buf_h = buf.height as usize;
            let cur_cols = (buf_w / 8).max(10);
            let cur_rows = (buf_h / 16).max(5);
            if cur_cols * cur_rows != cells.len() {
                cells.resize(cur_cols * cur_rows, TerminalCell::default());
            }

            if is_extras {
                if let Some(ref mut s) = extras_session {
                    if s.tick_and_draw(dt, &mut cells, cur_cols, cur_rows).is_err() {
                        is_extras = false;
                        ascii_saver.init(cur_cols, cur_rows);
                    }
                }
            }
            if !is_extras {
                ascii_saver.update(dt, cur_cols, cur_rows);
                ascii_saver.draw(&mut cells, cur_cols, cur_rows);
            }

            blit_cells_to_argb(&cells, cur_cols, cur_rows, buf.as_mut_slice(), buf_w, buf_h);

            if let Some(surf) = &state_wl.surface {
                surf.attach(Some(&buf.wl_buffer), 0, 0);
                surf.damage_buffer(0, 0, buf_w as i32, buf_h as i32);
                surf.commit();
            }
            connection.flush()?;

            current_buf ^= 1;
        }

        let elapsed = last_frame.elapsed();
        let sleep_dur = frame_dur.saturating_sub(elapsed);
        poll_and_read_events(&connection, &mut event_queue, &mut state_wl, sleep_dur)?;
    }

    if state_wl.exit_requested {
        state.stop();
    }

    // Cleanup resources
    if let Some(layer) = state_wl.layer_surface.take() {
        layer.destroy();
    }
    if let Some(toplevel) = state_wl.xdg_toplevel.take() {
        toplevel.destroy();
    }
    if let Some(xdg_surface) = state_wl.xdg_surface.take() {
        xdg_surface.destroy();
    }
    if let Some(surf) = state_wl.surface.take() {
        surf.destroy();
    }
    drop(buffers);
    let _ = event_queue.roundtrip(&mut state_wl);

    Ok(())
}
