// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! The wl_buffer release handler: clears the busy flag once the compositor
//! is done reading the buffer, so a retired buffer can be reclaimed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use wayland_client::protocol::wl_buffer;
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::wl_state::WaylandState;

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
