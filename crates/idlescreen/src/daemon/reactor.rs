// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Pure event-driven epoll reactor (0.00% CPU at idle, zero timers when user active),
//! monotonic suspend detector, and decoupled watchdog keeper thread.

#![allow(unsafe_code)]

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use super::notify::notify_watchdog;

pub const SUSPEND_THRESHOLD_NS: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockSample {
    pub boottime_ns: u64,
    pub monotonic_ns: u64,
}

pub fn read_clocks() -> ClockSample {
    let mut ts_boot = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let mut ts_mono = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    // SAFETY: `ts_boot` and `ts_mono` are valid stack-allocated `timespec` structs.
    unsafe {
        libc::clock_gettime(libc::CLOCK_BOOTTIME, &raw mut ts_boot);
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut ts_mono);
    }

    let boottime_ns = (ts_boot.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(ts_boot.tv_nsec as u64);
    let monotonic_ns = (ts_mono.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(ts_mono.tv_nsec as u64);

    ClockSample {
        boottime_ns,
        monotonic_ns,
    }
}

/// Checks if system suspended between two clock samples.
/// Returns (is_suspend, divergence_ns).
pub fn detect_suspend_divergence(
    prev: ClockSample,
    curr: ClockSample,
    threshold_ns: u64,
) -> (bool, u64) {
    let delta_boot = curr.boottime_ns.saturating_sub(prev.boottime_ns);
    let delta_mono = curr.monotonic_ns.saturating_sub(prev.monotonic_ns);
    let divergence = delta_boot.saturating_sub(delta_mono);
    (divergence >= threshold_ns, divergence)
}

pub struct WatchdogKeeper {
    last_heartbeat: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WatchdogKeeper {
    pub fn start() -> Self {
        let last_heartbeat = Arc::new(AtomicU64::new(read_clocks().monotonic_ns));
        let running = Arc::new(AtomicBool::new(true));

        let watchdog_usec = std::env::var("WATCHDOG_USEC")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .filter(|&u| u > 0);

        let thread = if let Some(usec) = watchdog_usec {
            let sleep_usec = (usec / 2).clamp(500_000, 30_000_000);
            let max_stale_ns = usec.saturating_mul(1_000);
            let hb = last_heartbeat.clone();
            let run = running.clone();

            std::thread::Builder::new()
                .name("idlescreen-wd".into())
                .spawn(move || {
                    while run.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_micros(sleep_usec));
                        if !run.load(Ordering::SeqCst) {
                            break;
                        }
                        let now_mono = read_clocks().monotonic_ns;
                        let last = hb.load(Ordering::SeqCst);
                        if now_mono.saturating_sub(last) < max_stale_ns {
                            notify_watchdog();
                        }
                    }
                })
                .ok()
        } else {
            None
        };

        Self {
            last_heartbeat,
            running,
            thread,
        }
    }

    pub fn record_heartbeat(&self) {
        self.last_heartbeat
            .store(read_clocks().monotonic_ns, Ordering::SeqCst);
    }
}

impl Drop for WatchdogKeeper {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub struct EpollReactor {
    epoll_fd: OwnedFd,
    watchdog: WatchdogKeeper,
    last_sample: ClockSample,
    pub last_poll_suspended: bool,
    pub running: bool,
}

impl EpollReactor {
    pub fn new() -> std::io::Result<Self> {
        // SAFETY: `epoll_create1(EPOLL_CLOEXEC)` creates a new epoll fd or returns -1.
        let raw = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if raw < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `raw` is a valid epoll file descriptor.
        let epoll_fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let watchdog = WatchdogKeeper::start();
        let last_sample = read_clocks();

        Ok(Self {
            epoll_fd,
            watchdog,
            last_sample,
            last_poll_suspended: false,
            running: true,
        })
    }

    pub fn add_fd(&self, fd: RawFd, events: u32, token: u64) -> std::io::Result<()> {
        let mut ev = libc::epoll_event { events, u64: token };
        // SAFETY: `self.epoll_fd` is valid and `ev` points to a valid `epoll_event`.
        let res = unsafe {
            libc::epoll_ctl(
                self.epoll_fd.as_raw_fd(),
                libc::EPOLL_CTL_ADD,
                fd,
                &raw mut ev,
            )
        };
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn remove_fd(&self, fd: RawFd) -> std::io::Result<()> {
        // SAFETY: `self.epoll_fd` is valid and `EPOLL_CTL_DEL` ignores the event pointer.
        let res = unsafe {
            libc::epoll_ctl(
                self.epoll_fd.as_raw_fd(),
                libc::EPOLL_CTL_DEL,
                fd,
                std::ptr::null_mut(),
            )
        };
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn poll_events(
        &mut self,
        events: &mut [libc::epoll_event],
        timeout_ms: i32,
    ) -> std::io::Result<usize> {
        // SAFETY: `events` is a valid mutable slice of `epoll_event`.
        let res = unsafe {
            libc::epoll_wait(
                self.epoll_fd.as_raw_fd(),
                events.as_mut_ptr(),
                events.len() as libc::c_int,
                timeout_ms,
            )
        };

        self.watchdog.record_heartbeat();

        let current_sample = read_clocks();
        let (suspended, _div) =
            detect_suspend_divergence(self.last_sample, current_sample, SUSPEND_THRESHOLD_NS);
        self.last_poll_suspended = suspended;
        if suspended {
            notify_watchdog();
        }
        self.last_sample = current_sample;

        if res < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                return Ok(0);
            }
            return Err(err);
        }

        Ok(res as usize)
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_clocks_and_suspend_divergence() {
        let prev = ClockSample {
            boottime_ns: 10_000_000_000,
            monotonic_ns: 10_000_000_000,
        };
        let curr = ClockSample {
            boottime_ns: 15_000_000_000,
            monotonic_ns: 10_050_000_000,
        };
        let (suspended, div) = detect_suspend_divergence(prev, curr, SUSPEND_THRESHOLD_NS);
        assert!(suspended);
        assert_eq!(div, 4_950_000_000);
    }
}
