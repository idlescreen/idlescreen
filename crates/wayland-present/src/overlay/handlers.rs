// SPDX-License-Identifier: MIT

use std::sync::atomic::Ordering;

use wayland_client::{
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_registry, wl_seat,
        wl_shm, wl_shm_pool, wl_surface,
    },
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

use crate::output::OutputLayout;

use super::state::{OutputTarget, SessionState};

impl Dispatch<wl_compositor::WlCompositor, ()> for SessionState {
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

impl Dispatch<wl_shm::WlShm, ()> for SessionState {
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

impl Dispatch<wl_shm_pool::WlShmPool, ()> for SessionState {
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

impl Dispatch<wl_buffer::WlBuffer, ()> for SessionState {
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

impl Dispatch<wl_registry::WlRegistry, ()> for SessionState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };

        match interface.as_str() {
            "wl_compositor" => {
                state.compositor = Some(registry.bind(name, version.min(4), queue, ()));
            }
            "wl_shm" => {
                state.shm = Some(registry.bind(name, version.min(1), queue, ()));
            }
            "zwlr_layer_shell_v1" => {
                state.layer_shell = Some(registry.bind(name, version.min(4), queue, ()));
            }
            "wl_output" => {
                let output = registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), queue, name);
                state.outputs.push(OutputTarget { id: name, output });
            }
            "wl_seat" if state.seat.is_none() => {
                let seat = registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(7), queue, ());
                state.pointer = Some(seat.get_pointer(queue, ()));
                seat.get_keyboard(queue, ());
                state.seat = Some(seat);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for SessionState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        output_id: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Geometry { x, y, .. } = event {
            state.output_origin.insert(*output_id, (x, y));
        }

        if let wl_output::Event::Mode { refresh, width, height, flags, .. } = event {
            let refresh_hz = (refresh.max(1000) / 1000) as u32;
            state
                .output_refresh_hz
                .insert(*output_id, refresh_hz.max(1));

            if matches!(flags, WEnum::Value(wl_output::Mode::Current)) {
                state.output_mode_size.insert(
                    *output_id,
                    (width.max(0) as u32, height.max(0) as u32),
                );
                if let Some(overlay) = state.overlays.get(output_id) {
                    let width = overlay.width.max(width.max(0) as u32);
                    let height = overlay.height.max(height.max(0) as u32);
                    if width > 0 && height > 0 {
                        let (x, y) = state.output_origin.get(output_id).copied().unwrap_or((0, 0));
                        state.output_registry.upsert(OutputLayout {
                            id: *output_id,
                            width,
                            height,
                            refresh_rate_hz: refresh_hz.max(1),
                            x,
                            y,
                        });
                    }
                }
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for SessionState {
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

impl Dispatch<wl_pointer::WlPointer, ()> for SessionState {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { serial, .. } => {
                state.pointer_serial = serial;
                if state.visible.load(Ordering::SeqCst) {
                    state.hide_pointer(serial);
                }
            }
            wl_pointer::Event::Motion { .. } => {
                if state.visible.load(Ordering::SeqCst) && state.pointer_serial != 0 {
                    state.hide_pointer(state.pointer_serial);
                }
                state.dismiss_from_input();
            }
            wl_pointer::Event::Button { .. } => {
                state.dismiss_from_input();
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for SessionState {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { .. } = event {
            state.dismiss_from_input();
        }
    }
}

impl Dispatch<wl_surface::WlSurface, u32> for SessionState {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for SessionState {
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

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, u32> for SessionState {
    fn event(
        state: &mut Self,
        _: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        output_id: &u32,
        _: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                state.configure_overlay(*output_id, serial, width, height);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.remove_overlay(*output_id);
            }
            _ => {}
        }

        let _ = queue;
    }
}