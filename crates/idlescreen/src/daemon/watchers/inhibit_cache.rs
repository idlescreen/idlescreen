// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Background snapshot of system/media idle inhibitors.
//!
//! Probing inhibitors shells out to `busctl`, `pactl` and `hyprctl`.
//! Those probes must never run on the event thread: each one forks a
//! process and waits for it, so a wedged audio server would block the
//! daemon — including its SIGTERM handling — for as long as the child
//! lives. The daemon therefore keeps a snapshot refreshed by a dedicated
//! thread and the event thread only ever reads the cached value.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::media_inhibit::probe_active_inhibitors;

/// How often the snapshot is refreshed.
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

static INHIBITED: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the refresher thread. Idempotent: later calls are no-ops.
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("idlescreen-inhibitors".into())
        .spawn(|| {
            loop {
                let inhibited = !probe_active_inhibitors().is_empty();
                INHIBITED.store(inhibited, Ordering::SeqCst);
                std::thread::sleep(REFRESH_INTERVAL);
            }
        });
}

/// Returns the cached inhibitor state.
///
/// Never blocks and never forks a process: safe to call from the event
/// thread and from the per-frame render path.
pub fn inhibited() -> bool {
    INHIBITED.load(Ordering::SeqCst)
}
