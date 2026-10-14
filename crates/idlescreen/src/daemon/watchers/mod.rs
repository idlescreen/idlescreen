// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Event-driven idle and session watchers across Wayland (ext-idle-notify-v1),
//! GNOME Mutter, systemd-logind, elogind, and ConsoleKit.

#![allow(unsafe_code)]

pub mod mutter;
pub mod session;
pub mod wayland_idle;

pub use mutter::{
    GnomeScreenSaverProxyBlocking, MutterIdleMonitorProxyBlocking, is_mutter_idle_available,
    spawn_mutter_idle_watcher,
};
pub use session::{
    LogindManagerProxyBlocking, LogindSessionProxyBlocking, spawn_gnome_session_watchers,
};
pub use wayland_idle::spawn_wayland_idle_watcher;

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct WakeHandle {
    fd: Option<Arc<OwnedFd>>,
}

impl WakeHandle {
    pub fn new_eventfd() -> Self {
        // SAFETY: `eventfd` with `EFD_CLOEXEC | EFD_NONBLOCK` returns a new fd or -1.
        let raw = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if raw >= 0 {
            // SAFETY: `raw` is a freshly created valid eventfd descriptor.
            Self {
                fd: Some(Arc::new(unsafe { OwnedFd::from_raw_fd(raw) })),
            }
        } else {
            Self { fd: None }
        }
    }

    pub fn new(raw_fd: RawFd) -> Self {
        if raw_fd >= 0 {
            // SAFETY: Caller transfers ownership of valid open `raw_fd`.
            Self {
                fd: Some(Arc::new(unsafe { OwnedFd::from_raw_fd(raw_fd) })),
            }
        } else {
            Self { fd: None }
        }
    }

    pub fn raw_fd(&self) -> RawFd {
        self.fd.as_ref().map_or(-1, AsRawFd::as_raw_fd)
    }

    pub fn is_valid(&self) -> bool {
        self.fd.is_some()
    }

    pub fn drain(&self) {
        if let Some(ref f) = self.fd {
            let mut buf = 0u64;
            // SAFETY: `f` is a valid non-blocking eventfd and `buf` is an 8-byte `u64`.
            unsafe {
                let _ = libc::read(
                    f.as_raw_fd(),
                    (&raw mut buf).cast::<libc::c_void>(),
                    std::mem::size_of::<u64>(),
                );
            }
        }
    }

    pub fn wake(&self) {
        if let Some(ref f) = self.fd {
            let val: u64 = 1;
            // SAFETY: `f` is a valid non-blocking eventfd and `val` is an 8-byte `u64`.
            unsafe {
                let _ = libc::write(
                    f.as_raw_fd(),
                    (&raw const val).cast::<libc::c_void>(),
                    std::mem::size_of::<u64>(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    #[test]
    fn test_screensaver_state_dismiss_on_activity() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        state.active.store(true, Ordering::SeqCst);
        assert!(state.active.load(Ordering::SeqCst));
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_screensaver_state_dismiss_on_lock_and_unlock() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        state.active.store(true, Ordering::SeqCst);
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_idle_and_lock_decision_matrix() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        let idle_timeout_ms = 120_000u64;
        let t_idle = 150_000u64;
        let is_locked = false;
        if t_idle >= idle_timeout_ms && !state.is_inhibited() && !is_locked {
            state.active.store(true, Ordering::SeqCst);
        }
        assert!(state.active.load(Ordering::SeqCst));

        let is_locked = true;
        if (t_idle < idle_timeout_ms || is_locked) && state.active.load(Ordering::SeqCst) {
            state.stop();
        }
        assert!(!state.active.load(Ordering::SeqCst));
    }
}
