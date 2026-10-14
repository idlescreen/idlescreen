// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Linux pidfd lifecycle management and non-blocking process reaping.

#![allow(unsafe_code)]

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::time::{Duration, Instant};

pub struct PidFd {
    fd: OwnedFd,
    pid: libc::pid_t,
}

impl PidFd {
    pub fn open(pid: libc::pid_t) -> std::io::Result<Self> {
        // SAFETY: SYS_pidfd_open takes a pid_t and flags (0); returns a new file descriptor or -1.
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) as RawFd };
        if raw < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self {
            // SAFETY: `raw` is a valid open file descriptor freshly returned by SYS_pidfd_open.
            fd: unsafe { OwnedFd::from_raw_fd(raw) },
            pid,
        })
    }

    pub fn pid(&self) -> libc::pid_t {
        self.pid
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    /// Attempts non-blocking reaping of child process via waitid on P_PIDFD.
    /// Returns Ok(Some(status)) if child exited, Ok(None) if still running, or Err.
    pub fn try_wait(&self) -> std::io::Result<Option<i32>> {
        // SAFETY: `siginfo_t` is a plain C struct safe to zero-initialize before `waitid`.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: `self.fd` is a valid pidfd and `info` points to writable `siginfo_t` storage.
        let res = unsafe {
            libc::waitid(
                libc::P_PIDFD,
                self.fd.as_raw_fd() as u32,
                &raw mut info,
                libc::WEXITED | libc::WNOHANG,
            )
        };
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }

        // SAFETY: `info` was initialized to zero and populated by `waitid`.
        let si_pid = unsafe { info.si_pid() };
        if si_pid != 0 {
            // SAFETY: When `si_pid != 0`, `si_status` holds the child exit status.
            let status = unsafe { info.si_status() };
            Ok(Some(status))
        } else {
            Ok(None)
        }
    }

    /// Reaps the process, waiting up to `timeout` for kernel exit completion after SIGKILL.
    pub fn wait_reap(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            match self.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    /// Sends a signal to the process referenced by this pidfd.
    pub fn send_signal(&self, sig: libc::c_int) -> std::io::Result<()> {
        // SAFETY: SYS_pidfd_send_signal takes a valid pidfd, signal number, null siginfo, and flags=0.
        let res = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.fd.as_raw_fd(),
                sig,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    /// Forcefully terminates the process with SIGKILL.
    pub fn kill(&self) -> std::io::Result<()> {
        self.send_signal(libc::SIGKILL)
    }
}
