// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Host-driven IPC client and supervisor for idlescreen-extras subprocess.

#![allow(unsafe_code)]

use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::Duration;

use super::breaker::{CircuitBreaker, is_circuit_breaker_tripped, record_circuit_breaker_failure};
use super::pidfd::PidFd;
use super::protocol::{IpcCommand, IpcResponse, send_fd};
pub use super::resolve::{DEFAULT_EXTRAS_BINARY, resolve_extras_binary};
use super::shm::{MAX_CELLS, ShmMapping, create_sealed_shm, create_socketpair};
use crate::cell::TerminalCell;

const REAP_TIMEOUT: Duration = Duration::from_millis(25);

pub struct ExtrasSession {
    saver_name: String,
    socket: UnixStream,
    pidfd: PidFd,
    _shm_fd: OwnedFd,
    shm: ShmMapping,
    cols: usize,
    rows: usize,
    pub breaker: CircuitBreaker,
}

impl ExtrasSession {
    pub fn is_available() -> bool {
        !is_circuit_breaker_tripped() && resolve_extras_binary().is_some()
    }

    pub fn spawn(saver_name: &str, cols: usize, rows: usize) -> std::io::Result<Self> {
        let count = cols.saturating_mul(rows);
        if cols == 0 || rows == 0 || count > MAX_CELLS || cols > 1000 || rows > 1000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "invalid grid dimensions: {cols}x{rows} (exceeds shared memory limit of {MAX_CELLS} cells)"
                ),
            ));
        }

        if is_circuit_breaker_tripped() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "extras circuit breaker is tripped due to repeated failures",
            ));
        }

        let bin_path = resolve_extras_binary().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "idlescreen-extras binary not found or not executable",
            )
        })?;

        let (parent_stream, child_fd) = create_socketpair()?;
        let (shm_fd, shm) = create_sealed_shm()?;

        let raw_child_fd = child_fd.as_raw_fd();
        let mut cmd = Command::new(&bin_path);
        cmd.arg(saver_name);
        cmd.env("IDLE_IPC_FD", "3");

        // SAFETY: `pre_exec` only invokes async-signal-safe `dup2`, `close`, and `fcntl` on `raw_child_fd`.
        unsafe {
            cmd.pre_exec(move || {
                if raw_child_fd != 3 {
                    if libc::dup2(raw_child_fd, 3) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    libc::close(raw_child_fd);
                }
                let flags = libc::fcntl(3, libc::F_GETFD);
                if flags >= 0 {
                    libc::fcntl(3, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
                }
                Ok(())
            });
        }

        let mut child = cmd.spawn()?;
        drop(child_fd);

        let child_pid = child.id() as libc::pid_t;
        let pidfd = match PidFd::open(child_pid) {
            Ok(p) => p,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        };

        if let Err(e) = send_fd(parent_stream.as_raw_fd(), shm_fd.as_raw_fd())
            .and_then(|()| parent_stream.set_read_timeout(Some(Duration::from_millis(500))))
            .and_then(|()| IpcResponse::read_from(&parent_stream))
            .and_then(|resp| {
                if resp == IpcResponse::Ready {
                    Ok(())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "expected Ready response from child",
                    ))
                }
            })
        {
            let _ = pidfd.kill();
            pidfd.wait_reap(REAP_TIMEOUT);
            let _ = child.wait();
            return Err(e);
        }

        let mut session = Self {
            saver_name: saver_name.to_string(),
            socket: parent_stream,
            pidfd,
            _shm_fd: shm_fd,
            shm,
            cols: 0,
            rows: 0,
            breaker: CircuitBreaker::new(),
        };

        session.init_grid(cols, rows)?;
        Ok(session)
    }

    pub fn saver_name(&self) -> &str {
        &self.saver_name
    }

    pub fn pidfd(&self) -> &PidFd {
        &self.pidfd
    }

    fn abort_child(&mut self) {
        let _ = self.pidfd.kill();
        self.pidfd.wait_reap(REAP_TIMEOUT);
        record_circuit_breaker_failure();
        self.breaker.record_failure();
    }

    pub fn init_grid(&mut self, cols: usize, rows: usize) -> std::io::Result<()> {
        let max_cells = self.shm.len / std::mem::size_of::<TerminalCell>();
        let count = cols.saturating_mul(rows);
        if cols == 0 || rows == 0 || count > max_cells || cols > 1000 || rows > 1000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid grid dimensions: {cols}x{rows}"),
            ));
        }

        let cmd = IpcCommand::Init {
            cols: cols as u32,
            rows: rows as u32,
        };
        if let Err(e) = cmd.write_to(&mut self.socket) {
            self.abort_child();
            return Err(e);
        }

        self.socket
            .set_read_timeout(Some(Duration::from_millis(500)))?;
        match IpcResponse::read_from(&mut self.socket) {
            Ok(IpcResponse::Ack) => {
                self.cols = cols;
                self.rows = rows;
                Ok(())
            }
            Ok(_) => {
                self.abort_child();
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "expected Ack response from child Init",
                ))
            }
            Err(e) => {
                self.abort_child();
                Err(e)
            }
        }
    }

    pub fn tick_and_draw(
        &mut self,
        dt: Duration,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
    ) -> std::io::Result<bool> {
        if is_circuit_breaker_tripped() || self.breaker.is_tripped() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "circuit breaker is tripped",
            ));
        }

        if cols != self.cols || rows != self.rows {
            self.init_grid(cols, rows)?;
        }

        let cmd = IpcCommand::TickAndDraw {
            dt_micros: dt.as_micros() as u64,
        };
        if let Err(e) = cmd.write_to(&mut self.socket) {
            self.abort_child();
            return Err(e);
        }

        self.socket
            .set_read_timeout(Some(Duration::from_millis(500)))?;
        match IpcResponse::read_from(&mut self.socket) {
            Ok(IpcResponse::FrameReady { dirty, .. }) => {
                let count = cols.saturating_mul(rows);
                // SAFETY: `self.shm.ptr` is mapped for `MAX_CELLS` cells and `count <= MAX_CELLS`.
                let slice = unsafe { std::slice::from_raw_parts(self.shm.ptr, count) };
                let n = count.min(grid.len());
                grid[..n].copy_from_slice(&slice[..n]);
                Ok(dirty)
            }
            Ok(_) => {
                self.abort_child();
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unexpected response to TickAndDraw",
                ))
            }
            Err(e) => {
                self.abort_child();
                Err(e)
            }
        }
    }
}

impl Drop for ExtrasSession {
    fn drop(&mut self) {
        let _ = IpcCommand::Stop.write_to(&mut self.socket);
        let _ = self.pidfd.kill();
        self.pidfd.wait_reap(REAP_TIMEOUT);
    }
}
