// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use crate::wayland;

/// Tracks user inactivity through the Wayland `ext-idle-notify-v1` protocol.
///
/// Returns `None` from [`Self::new`] when `WAYLAND_DISPLAY` is unset or the
/// compositor does not expose the idle notifier.
pub struct IdleMonitor {
    is_idle: Arc<AtomicBool>,
    timeout_tx: mpsc::Sender<u32>,
    shutdown: Arc<AtomicBool>,
}

impl IdleMonitor {
    /// Connect to the current Wayland session and begin monitoring idle state.
    pub fn new(timeout_mins: u32) -> Option<Self> {
        if !Self::is_available() {
            return None;
        }

        let is_idle = Arc::new(AtomicBool::new(false));
        let (timeout_tx, timeout_rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));

        wayland::spawn_event_thread(
            is_idle.clone(),
            shutdown.clone(),
            timeout_rx,
            timeout_mins,
        );

        Some(Self {
            is_idle,
            timeout_tx,
            shutdown,
        })
    }

    /// Whether `WAYLAND_DISPLAY` is set in the environment.
    pub fn is_available() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    /// Returns `true` when the user has been idle longer than the configured timeout.
    pub fn is_idle(&self) -> bool {
        self.is_idle.load(Ordering::SeqCst)
    }

    /// Update the idle timeout. The compositor is re-notified on the next event-loop tick.
    pub fn set_timeout(&self, timeout_mins: u32) {
        let _ = self.timeout_tx.send(timeout_mins);
    }
}

impl Drop for IdleMonitor {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}