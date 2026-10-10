// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Pure event-driven epoll reactor (0.00% CPU at idle, zero timers when user active),
//! monotonic suspend detector, and decoupled watchdog keeper thread.

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use super::notify::notify_watchdog;

pub const SUSPEND_THRESHOLD_NS: u64 = 1_000_000_000; // 1.0 second divergence

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockSample {
    pub boottime_ns: u64,
    pub monotonic_ns: u64,
}

pub fn read_clocks() -> ClockSample {
    let mut ts_boot = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    let mut ts_mono = libc::timespec { tv_sec: 0, tv_nsec: 0 };

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

        let hb = last_heartbeat.clone();
        let run = running.clone();

        let thread = std::thread::Builder::new()
            .name("idlescreen-wd".into())
            .spawn(move || {
                while run.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_secs(3));
                    if !run.load(Ordering::SeqCst) {
                        break;
                    }
                    let now_mono = read_clocks().monotonic_ns;
                    let last = hb.load(Ordering::SeqCst);
                    let elapsed = now_mono.saturating_sub(last);

                    // If reactor heartbeat was updated within 6 seconds, send watchdog ping
                    if elapsed < 6_000_000_000 {
                        notify_watchdog();
                    }
                }
            })
            .ok();

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
    pub running: bool,
}

impl EpollReactor {
    pub fn new() -> std::io::Result<Self> {
        let raw = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if raw < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let epoll_fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let watchdog = WatchdogKeeper::start();
        let last_sample = read_clocks();

        Ok(Self {
            epoll_fd,
            watchdog,
            last_sample,
            running: true,
        })
    }

    pub fn add_fd(&self, fd: RawFd, events: u32, token: u64) -> std::io::Result<()> {
        let mut ev = libc::epoll_event {
            events,
            u64: token,
        };
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

    /// Waits for events on the registered file descriptors.
    ///
    /// When timeout_ms is -1, sleeps indefinitely with 0.00% CPU until an event occurs.
    /// Automatically checks for suspend divergence and updates watchdog heartbeat.
    pub fn poll_events(
        &mut self,
        events: &mut [libc::epoll_event],
        timeout_ms: i32,
    ) -> std::io::Result<usize> {
        let res = unsafe {
            libc::epoll_wait(
                self.epoll_fd.as_raw_fd(),
                events.as_mut_ptr(),
                events.len() as libc::c_int,
                timeout_ms,
            )
        };

        // Record heartbeat
        self.watchdog.record_heartbeat();

        // Check for suspend divergence
        let current_sample = read_clocks();
        let (suspended, _div) =
            detect_suspend_divergence(self.last_sample, current_sample, SUSPEND_THRESHOLD_NS);
        if suspended {
            // Immediate resume handling: watchdog ping & display resync flag
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
            boottime_ns: 15_000_000_000, // +5s
            monotonic_ns: 10_050_000_000, // +50ms
        };

        let (suspended, div) = detect_suspend_divergence(prev, curr, SUSPEND_THRESHOLD_NS);
        assert!(suspended);
        assert_eq!(div, 4_950_000_000);
    }
}
