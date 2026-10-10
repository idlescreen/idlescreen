// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Host-driven IPC client and supervisor for idlescreen-extras subprocess.

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::breaker::{
    CircuitBreaker, is_circuit_breaker_tripped, record_circuit_breaker_failure,
};
use super::pidfd::PidFd;
use super::protocol::{IpcCommand, IpcResponse, send_fd};
use crate::cell::TerminalCell;

pub const DEFAULT_EXTRAS_BINARY: &str = "/usr/libexec/idlescreen/idlescreen-extras";

pub struct ExtrasSession {
    saver_name: String,
    socket: UnixStream,
    pidfd: PidFd,
    _shm_fd: OwnedFd,
    shm_ptr: *mut TerminalCell,
    shm_len: usize,
    cols: usize,
    rows: usize,
    pub breaker: CircuitBreaker,
}

impl ExtrasSession {
    /// Checks if the idlescreen-extras binary exists and is executable,
    /// and the circuit breaker has not been tripped.
    pub fn is_available() -> bool {
        !is_circuit_breaker_tripped() && resolve_extras_binary().is_some()
    }

    /// Spawns a new extras screensaver subprocess.
    pub fn spawn(saver_name: &str, cols: usize, rows: usize) -> std::io::Result<Self> {
        let max_cells = 512 * 256;
        let count = cols.saturating_mul(rows);
        if cols == 0 || rows == 0 || count > max_cells || cols > 1000 || rows > 1000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid grid dimensions: {cols}x{rows} (exceeds shared memory limit of {max_cells} cells)"),
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

        // 1. Create anonymous socketpair (SOCK_STREAM or SOCK_SEQPACKET with CLOEXEC)
        let mut fds = [0 as RawFd; 2];
        let res = unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        };
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let parent_stream = unsafe { UnixStream::from_raw_fd(fds[0]) };
        let child_fd = fds[1];

        // 2. Create anonymous shared memory via memfd_create
        let memfd_name = c"idlescreen-extras-shm";
        let raw_shm = unsafe {
            libc::syscall(
                libc::SYS_memfd_create,
                memfd_name.as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
            ) as RawFd
        };
        if raw_shm < 0 {
            unsafe { libc::close(child_fd) };
            return Err(std::io::Error::last_os_error());
        }

        // Allocate shared memory for grid (up to 512x256 max cells)
        let max_cells = 512 * 256;
        let shm_len = max_cells * std::mem::size_of::<TerminalCell>();
        if unsafe { libc::ftruncate(raw_shm, shm_len as libc::off_t) } < 0 {
            unsafe {
                libc::close(child_fd);
                libc::close(raw_shm);
            }
            return Err(std::io::Error::last_os_error());
        }

        // Seal the shared memory to prevent shrinking/growing
        let seals = libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_SEAL;
        if unsafe { libc::fcntl(raw_shm, libc::F_ADD_SEALS, seals) } < 0 {
            unsafe {
                libc::close(child_fd);
                libc::close(raw_shm);
            }
            return Err(std::io::Error::last_os_error());
        }

        let shm_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                shm_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                raw_shm,
                0,
            )
        };
        if shm_ptr == libc::MAP_FAILED {
            unsafe {
                libc::close(child_fd);
                libc::close(raw_shm);
            }
            return Err(std::io::Error::other(
                "failed to mmap shared memory in parent",
            ));
        }

        let shm_fd = unsafe { OwnedFd::from_raw_fd(raw_shm) };

        // 3. Spawn child with child_fd cleared of CLOEXEC
        let mut cmd = Command::new(&bin_path);
        cmd.arg(saver_name);
        cmd.env("IDLE_IPC_FD", "3");

        unsafe {
            cmd.pre_exec(move || {
                // Duplicate child_fd onto fd 3
                if child_fd != 3 {
                    if libc::dup2(child_fd, 3) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    libc::close(child_fd);
                }
                // Clear FD_CLOEXEC on fd 3
                let flags = libc::fcntl(3, libc::F_GETFD);
                if flags >= 0 {
                    libc::fcntl(3, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
                }
                Ok(())
            });
        }

        let child = cmd.spawn()?;
        // Close child end in parent
        unsafe { libc::close(child_fd) };

        let child_pid = child.id() as libc::pid_t;
        let pidfd = PidFd::open(child_pid)?;

        // 4. Send shared memory file descriptor to child via SCM_RIGHTS
        send_fd(parent_stream.as_raw_fd(), shm_fd.as_raw_fd())?;

        // 5. Read IpcResponse::Ready from child
        parent_stream.set_read_timeout(Some(Duration::from_millis(500)))?;
        let resp = IpcResponse::read_from(&parent_stream)?;
        if resp != IpcResponse::Ready {
            let _ = pidfd.kill();
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "expected Ready response from child",
            ));
        }

        let mut session = Self {
            saver_name: saver_name.to_string(),
            socket: parent_stream,
            pidfd,
            _shm_fd: shm_fd,
            shm_ptr: shm_ptr.cast(),
            shm_len,
            cols: 0,
            rows: 0,
            breaker: CircuitBreaker::new(),
        };

        // 6. Initialize grid dimensions
        session.init_grid(cols, rows)?;

        Ok(session)
    }

    pub fn saver_name(&self) -> &str {
        &self.saver_name
    }

    pub fn pidfd(&self) -> &PidFd {
        &self.pidfd
    }

    pub fn init_grid(&mut self, cols: usize, rows: usize) -> std::io::Result<()> {
        let max_cells = self.shm_len / std::mem::size_of::<TerminalCell>();
        let count = cols.saturating_mul(rows);
        if cols == 0 || rows == 0 || count > max_cells || cols > 1000 || rows > 1000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid grid dimensions: {cols}x{rows} (exceeds shared memory limit of {max_cells} cells)"),
            ));
        }

        let cmd = IpcCommand::Init {
            cols: cols as u32,
            rows: rows as u32,
        };
        if let Err(e) = cmd.write_to(&mut self.socket) {
            let _ = self.pidfd.kill();
            let _ = self.pidfd.try_wait();
            record_circuit_breaker_failure();
            self.breaker.record_failure();
            return Err(e);
        }

        self.socket.set_read_timeout(Some(Duration::from_millis(500)))?;
        let resp = match IpcResponse::read_from(&mut self.socket) {
            Ok(r) => r,
            Err(e) => {
                let _ = self.pidfd.kill();
                let _ = self.pidfd.try_wait();
                record_circuit_breaker_failure();
                self.breaker.record_failure();
                return Err(e);
            }
        };

        if resp != IpcResponse::Ack {
            let _ = self.pidfd.kill();
            let _ = self.pidfd.try_wait();
            record_circuit_breaker_failure();
            self.breaker.record_failure();
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "expected Ack response from child Init",
            ));
        }

        self.cols = cols;
        self.rows = rows;
        Ok(())
    }

    /// Sends TickAndDraw to child with a 500ms deadline timer.
    /// On success, copies the rendered cells into `grid`.
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
            let _ = self.pidfd.kill();
            let _ = self.pidfd.try_wait();
            record_circuit_breaker_failure();
            self.breaker.record_failure();
            return Err(e);
        }

        // 500ms deadline timeout
        self.socket.set_read_timeout(Some(Duration::from_millis(500)))?;
        match IpcResponse::read_from(&mut self.socket) {
            Ok(IpcResponse::FrameReady { dirty, .. }) => {
                let count = cols.saturating_mul(rows);
                let slice = unsafe { std::slice::from_raw_parts(self.shm_ptr, count) };
                let n = count.min(grid.len());
                grid[..n].copy_from_slice(&slice[..n]);
                Ok(dirty)
            }
            Ok(_) => {
                let _ = self.pidfd.kill();
                let _ = self.pidfd.try_wait();
                record_circuit_breaker_failure();
                self.breaker.record_failure();
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unexpected response to TickAndDraw",
                ))
            }
            Err(e) => {
                // Trip breaker on timeout or error, kill child and reap
                let _ = self.pidfd.kill();
                let _ = self.pidfd.try_wait();
                record_circuit_breaker_failure();
                self.breaker.record_failure();
                Err(e)
            }
        }
    }
}

impl Drop for ExtrasSession {
    fn drop(&mut self) {
        let _ = IpcCommand::Stop.write_to(&mut self.socket);
        let _ = self.pidfd.kill();
        let _ = self.pidfd.try_wait();
        if !self.shm_ptr.is_null() && self.shm_len > 0 {
            unsafe {
                libc::munmap(self.shm_ptr.cast(), self.shm_len);
            }
        }
    }
}

/// Resolves path to idlescreen-extras binary.
pub fn resolve_extras_binary() -> Option<PathBuf> {
    // 1. Check default installation location
    let def = Path::new(DEFAULT_EXTRAS_BINARY);
    if is_executable(def) {
        return Some(def.to_path_buf());
    }

    // 2. Check CARGO_BIN_EXE_idlescreen-extras or current exe directory (for tests)
    if let Ok(current) = std::env::current_exe()
        && let Some(dir) = current.parent() {
            let candidate = dir.join("idlescreen-extras");
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }

    // 3. Check PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("idlescreen-extras");
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = path.metadata() {
        meta.is_file() && (meta.permissions().mode() & 0o111 != 0)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::breaker::reset_circuit_breaker;

    #[test]
    fn test_spawn_oversized_grid_rejected_immediately() {
        let res = ExtrasSession::spawn("storm", 600, 300);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_spawn_zero_dimension_grid_rejected_immediately() {
        let res = ExtrasSession::spawn("storm", 0, 100);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), std::io::ErrorKind::InvalidInput);

        let res2 = ExtrasSession::spawn("storm", 100, 0);
        assert!(res2.is_err());
        assert_eq!(res2.err().unwrap().kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_circuit_breaker_tripping_blocks_availability_and_spawn() {
        reset_circuit_breaker();
        assert!(!is_circuit_breaker_tripped());

        record_circuit_breaker_failure();
        assert!(!is_circuit_breaker_tripped());

        record_circuit_breaker_failure();
        assert!(!is_circuit_breaker_tripped());

        record_circuit_breaker_failure();
        assert!(is_circuit_breaker_tripped());
        assert!(!ExtrasSession::is_available());

        let res = ExtrasSession::spawn("storm", 80, 24);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), std::io::ErrorKind::BrokenPipe);

        reset_circuit_breaker();
        assert!(!is_circuit_breaker_tripped());
    }
}
