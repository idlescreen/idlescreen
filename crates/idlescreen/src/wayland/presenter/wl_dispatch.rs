// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Wayland protocol event dispatch implementations for `WaylandState`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_registry, wl_seat, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use super::{DetectedOutput, WaylandState};

macro_rules! impl_dispatch {
    ($ty:ty, ($state:ident, $proxy:ident, $event:ident, $qh:ident) => $body:block) => {
        impl Dispatch<$ty, ()> for WaylandState {
            fn event(
                $state: &mut Self,
                $proxy: &$ty,
                $event: <$ty as wayland_client::Proxy>::Event,
                (): &(),
                _: &Connection,
                $qh: &QueueHandle<Self>,
            ) $body
        }
    };
}

impl_dispatch!(wl_registry::WlRegistry, (state, registry, event, qh) => {
    if let wl_registry::Event::Global { name, interface, version } = event {
        match interface.as_str() {
            "wl_compositor" => state.compositor = Some(registry.bind(name, version.min(4), qh, ())),
            "wl_shm" => state.shm = Some(registry.bind(name, version.min(1), qh, ())),
            "wp_viewporter" => state.viewporter = Some(registry.bind(name, version.min(1), qh, ())),
            "zwlr_layer_shell_v1" => state.layer_shell = Some(registry.bind(name, version.min(4), qh, ())),
            "xdg_wm_base" => state.xdg_wm_base = Some(registry.bind(name, version.min(2), qh, ())),
            "wl_output" => {
                let output: wl_output::WlOutput = registry.bind(name, version.min(4), qh, ());
                if state.output.is_none() {
                    state.output = Some(output.clone());
                }
                state.outputs.push(DetectedOutput {
                    name,
                    connector_name: String::new(),
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
});

impl_dispatch!(wl_output::WlOutput, (state, output, event, _qh) => {
    match event {
        wl_output::Event::Name { name } => {
            if let Some(out) = state.outputs.iter_mut().find(|o| &o.output == output) {
                out.connector_name = name;
            }
        }
        wl_output::Event::Mode { flags, width, height, refresh, .. } => {
            let is_current = match flags {
                WEnum::Value(wl_output::Mode::Current) => true,
                WEnum::Unknown(val) => (val & 1) != 0,
                WEnum::Value(_) => false,
            };
            if is_current
                && let (Ok(w), Ok(h)) = (u32::try_from(width), u32::try_from(height))
                && w > 0
                && h > 0
            {
                if let Some(out) = state.outputs.iter_mut().find(|o| &o.output == output) {
                    (out.width, out.height, out.refresh_mhz) = (w, h, refresh);
                }
                for s in state.surfaces.iter_mut().filter(|s| s.output.as_ref() == Some(output)) {
                    (s.width, s.height, s.refresh_mhz) = (w, h, refresh);
                }
                if state.output.as_ref() == Some(output) || state.output_width == 0 {
                    (state.output_width, state.output_height, state.output_refresh_mhz) = (w, h, refresh);
                }
            }
        }
        _ => {}
    }
});

impl_dispatch!(xdg_wm_base::XdgWmBase, (_s, wm_base, event, _qh) => {
    if let xdg_wm_base::Event::Ping { serial } = event {
        wm_base.pong(serial);
    }
});

impl_dispatch!(xdg_surface::XdgSurface, (state, surface, event, _qh) => {
    if let xdg_surface::Event::Configure { serial } = event {
        surface.ack_configure(serial);
        if let Some(surf) = state.surfaces.iter_mut().find(|s| s.xdg_surface.as_ref() == Some(surface)) {
            surf.configured = true;
        }
        state.configured = true;
    }
});

impl_dispatch!(xdg_toplevel::XdgToplevel, (state, toplevel, event, _qh) => {
    match event {
        xdg_toplevel::Event::Configure { width, height, .. } => {
            if let (Ok(w), Ok(h)) = (u32::try_from(width), u32::try_from(height))
                && w > 0
                && h > 0
            {
                if let Some(s) = state.surfaces.iter_mut().find(|s| s.xdg_toplevel.as_ref() == Some(toplevel)) {
                    (s.configured_width, s.configured_height) = (w, h);
                }
                if state.configured_width == 0 {
                    (state.configured_width, state.configured_height) = (w, h);
                }
            }
        }
        xdg_toplevel::Event::Close => state.exit_requested = true,
        _ => {}
    }
});

impl_dispatch!(zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, (state, layer_surface, event, _qh) => {
    match event {
        zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
            layer_surface.ack_configure(serial);
            if let Some(s) = state.surfaces.iter_mut().find(|s| s.layer_surface.as_ref() == Some(layer_surface)) {
                s.configured = true;
                if width > 0 && height > 0 {
                    (s.configured_width, s.configured_height) = (width, height);
                }
            }
            state.configured = true;
            if width > 0 && height > 0 && state.configured_width == 0 {
                (state.configured_width, state.configured_height) = (width, height);
            }
        }
        zwlr_layer_surface_v1::Event::Closed => state.exit_requested = true,
        _ => {}
    }
});

impl_dispatch!(wl_keyboard::WlKeyboard, (state, _kb, event, _qh) => {
    if let wl_keyboard::Event::Key { state: key_state, .. } = event {
        state.handle_key_event(key_state);
    }
});

impl_dispatch!(wl_pointer::WlPointer, (state, _ptr, event, _qh) => {
    match event {
        wl_pointer::Event::Enter { surface_x, surface_y, .. } => state.handle_pointer_enter(surface_x, surface_y),
        wl_pointer::Event::Button { state: btn_state, .. } => state.handle_pointer_button(btn_state),
        wl_pointer::Event::Motion { surface_x, surface_y, .. } => state.handle_pointer_motion(surface_x, surface_y),
        wl_pointer::Event::Axis { .. } => state.handle_pointer_axis(),
        _ => {}
    }
});

impl Dispatch<wl_buffer::WlBuffer, Arc<AtomicBool>> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        busy: &Arc<AtomicBool>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_buffer::Event::Release) {
            busy.store(false, Ordering::Release);
        }
    }
}

macro_rules! impl_empty_dispatch {
    ($($ty:ty),+ $(,)?) => {
        $(impl_dispatch!($ty, (_s, _p, _e, _q) => {});)+
    };
}

impl_empty_dispatch!(
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    wl_compositor::WlCompositor,
    wl_shm::WlShm,
    wl_shm_pool::WlShmPool,
    wl_surface::WlSurface,
    wl_seat::WlSeat,
    WpViewporter,
    WpViewport,
);
