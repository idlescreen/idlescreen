// SPDX-License-Identifier: MIT

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    protocol::{wl_output, wl_registry, wl_seat},
};

use crate::output::OutputLayout;

use super::super::state::{OutputTarget, SessionState};

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
        tracing::debug!(%interface, name, "wl_registry global");

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
            "wp_viewporter" => {
                state.viewporter = Some(registry.bind(name, version.min(1), queue, ()));
            }
            "wl_output" => {
                let output =
                    registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), queue, name);
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
    #[tracing::instrument(skip_all, fields(output_id))]
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

        if let wl_output::Event::Scale { factor } = event {
            state.output_scale.insert(*output_id, factor);
        }

        if let wl_output::Event::Mode {
            refresh,
            width,
            height,
            flags,
            ..
        } = event
        {
            let refresh_hz = (refresh.max(1000) / 1000) as u32;
            state
                .output_refresh_hz
                .insert(*output_id, refresh_hz.max(1));

            if matches!(flags, WEnum::Value(wl_output::Mode::Current)) {
                state
                    .output_mode_size
                    .insert(*output_id, (width.max(0) as u32, height.max(0) as u32));
                if let Some(overlay) = state.overlays.get(output_id) {
                    let width = overlay.width.max(width.max(0) as u32);
                    let height = overlay.height.max(height.max(0) as u32);
                    if width > 0 && height > 0 {
                        let (x, y) = state
                            .output_origin
                            .get(output_id)
                            .copied()
                            .unwrap_or((0, 0));
                        let scale = state
                            .output_scale
                            .get(output_id)
                            .copied()
                            .unwrap_or(1);
                        state.output_registry.upsert(OutputLayout {
                            id: *output_id,
                            width,
                            height,
                            refresh_rate_hz: refresh_hz.max(1),
                            x,
                            y,
                            scale,
                        });
                    }
                }
            }
        }
    }
}

impl Dispatch<wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter, ()>
    for SessionState
{
    fn event(
        _: &mut Self,
        _: &wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter,
        _: wayland_protocols::wp::viewporter::client::wp_viewporter::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport, ()>
    for SessionState
{
    fn event(
        _: &mut Self,
        _: &wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport,
        _: wayland_protocols::wp::viewporter::client::wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
