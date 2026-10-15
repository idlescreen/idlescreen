// SPDX-License-Identifier: Apache-2.0
//! Readiness notification socket and pipe probe.

#![allow(unsafe_code)]

use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

pub struct NotifySocketProbe {
    _temp_dir: TempDir,
    socket_path: PathBuf,
    listener: UnixDatagram,
}

impl Default for NotifySocketProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl NotifySocketProbe {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("create temp dir for notify socket");
        let socket_path = temp_dir.path().join("notify.sock");
        let listener = UnixDatagram::bind(&socket_path).expect("bind notify socket");
        listener
            .set_read_timeout(Some(Duration::from_millis(500)))
            .ok();

        Self {
            _temp_dir: temp_dir,
            socket_path,
            listener,
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn recv_message(&self) -> Option<String> {
        let mut buf = [0u8; 512];
        match self.listener.recv(&mut buf) {
            Ok(n) if n > 0 => String::from_utf8(buf[..n].to_vec()).ok(),
            _ => None,
        }
    }
}

pub fn emit_notify_socket(path: &Path, message: &str) -> std::io::Result<()> {
    let sock = UnixDatagram::unbound()?;
    sock.send_to(message.as_bytes(), path)?;
    Ok(())
}

pub struct PipeReadinessProbe {
    read_pipe: std::fs::File,
    write_pipe: Option<std::fs::File>,
    raw_write_fd: i32,
}

impl Default for PipeReadinessProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl PipeReadinessProbe {
    pub fn new() -> Self {
        let mut fds = [0i32; 2];
        // SAFETY: `fds` is a valid 2-element `i32` array for `libc::pipe`.
        let res = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(res, 0, "libc::pipe must succeed");

        // SAFETY: `fds[0]` is a freshly opened read pipe descriptor owned by this struct.
        let read_pipe = unsafe { std::fs::File::from_raw_fd(fds[0]) };
        // SAFETY: `fds[1]` is a freshly opened write pipe descriptor owned by this struct.
        let write_pipe = unsafe { std::fs::File::from_raw_fd(fds[1]) };
        let raw_write_fd = write_pipe.as_raw_fd();

        Self {
            read_pipe,
            write_pipe: Some(write_pipe),
            raw_write_fd,
        }
    }

    pub fn take_write_fd(&self) -> i32 {
        self.raw_write_fd
    }

    pub fn simulate_daemon_ready(&mut self) -> std::io::Result<()> {
        if let Some(mut file) = self.write_pipe.take() {
            file.write_all(b"\n")?;
            file.flush()?;
            drop(file);
        }
        Ok(())
    }

    pub fn await_readiness(&mut self) -> Result<bool, String> {
        let mut buf = [0u8; 16];
        let n = self
            .read_pipe
            .read(&mut buf)
            .map_err(|e| format!("read failed: {e}"))?;

        if n > 0 && buf[0] == b'\n' {
            // Check EOF after readiness byte
            let mut eof_buf = [0u8; 1];
            let extra = self.read_pipe.read(&mut eof_buf).unwrap_or(0);
            Ok(extra == 0)
        } else {
            Ok(false)
        }
    }
}
