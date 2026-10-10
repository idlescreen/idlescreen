// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Linux pidfd lifecycle management and non-blocking process reaping.

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

pub struct PidFd {
    fd: OwnedFd,
    pid: libc::pid_t,
}

impl PidFd {
    pub fn open(pid: libc::pid_t) -> std::io::Result<Self> {
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) as RawFd };
        if raw < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self {
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
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
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

        // If child exited, si_pid will be non-zero
        let si_pid = unsafe { info.si_pid() };
        if si_pid != 0 {
            let status = unsafe { info.si_status() };
            Ok(Some(status))
        } else {
            Ok(None)
        }
    }

    /// Sends a signal to the process referenced by this pidfd.
    pub fn send_signal(&self, sig: libc::c_int) -> std::io::Result<()> {
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
