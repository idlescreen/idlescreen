// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Multi-protocol readiness notifications (systemd NOTIFY_SOCKET, fd 3 pipe, D-Bus).

#![allow(unsafe_code)]

use std::os::unix::net::UnixDatagram;
use std::path::Path;

/// Sends a state notification string via systemd NOTIFY_SOCKET.
///
/// Supports both filesystem socket paths and abstract Linux sockets (prefixed by `@`).
pub fn sd_notify(state: &str) -> std::io::Result<bool> {
    let notify_socket = match std::env::var("NOTIFY_SOCKET") {
        Ok(s) if !s.trim().is_empty() => s,
        _ => return Ok(false),
    };

    let sock = UnixDatagram::unbound()?;
    let payload = state.as_bytes();

    if let Some(abstract_path) = notify_socket.strip_prefix('@') {
        // SAFETY: `sockaddr_un` is a plain C struct safe to zero-initialize.
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as libc::sa_family_t;

        let bytes = abstract_path.as_bytes();
        let max_len = addr.sun_path.len() - 1;
        let copy_len = bytes.len().min(max_len);

        addr.sun_path[0] = 0;
        for i in 0..copy_len {
            addr.sun_path[1 + i] = bytes[i] as libc::c_char;
        }

        let addr_len = (std::mem::size_of::<libc::sa_family_t>() + 1 + copy_len) as libc::socklen_t;
        use std::os::fd::AsRawFd;
        // SAFETY: `sock` is valid and `addr` is properly sized for the abstract socket path.
        let res = unsafe {
            libc::sendto(
                sock.as_raw_fd(),
                payload.as_ptr().cast(),
                payload.len(),
                0,
                std::ptr::addr_of!(addr).cast(),
                addr_len,
            )
        };
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }
    } else {
        sock.send_to(payload, Path::new(&notify_socket))?;
    }

    Ok(true)
}

/// Signals readiness via an open FIFO/pipe (standard s6 / dinit readiness protocol).
pub fn notify_fd(fd: libc::c_int) -> bool {
    // SAFETY: `libc::stat` is a plain C struct safe to zero-initialize.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: `fstat` safely probes `fd` and returns -1 on EBADF.
    if unsafe { libc::fstat(fd, &raw mut st) } < 0 {
        return false;
    }

    if (st.st_mode & libc::S_IFMT) != libc::S_IFIFO {
        return false;
    }

    let byte = *b"\n";
    // SAFETY: `fd` was verified to be an open FIFO and `byte` is a 1-byte buffer.
    let res = unsafe { libc::write(fd, (&raw const byte).cast(), 1) };
    if res > 0 {
        // SAFETY: Closing the s6/dinit readiness FIFO completes the readiness handshake.
        unsafe { libc::close(fd) };
        true
    } else {
        false
    }
}

/// Signals readiness via fd 3 (standard s6 / dinit readiness protocol).
pub fn notify_fd3() -> bool {
    notify_fd(3)
}

/// Signals overall daemon readiness across all protocols.
pub fn notify_ready() {
    let _ = sd_notify("READY=1\n");
    let _ = notify_fd3();
}

/// Sends a watchdog heartbeat to the supervisor.
pub fn notify_watchdog() {
    let _ = sd_notify("WATCHDOG=1\n");
}

/// Signals daemon shutdown to the supervisor.
pub fn notify_stopping() {
    let _ = sd_notify("STOPPING=1\n");
}

/// Emits a daemon status message via `STATUS=` on `NOTIFY_SOCKET` and POSIX `syslog`.
pub fn log_status(status: &str) {
    let _ = sd_notify(&format!("STATUS={status}\n"));
    let clean = status.replace('\0', " ");
    if let Ok(c_msg) = std::ffi::CString::new(clean) {
        // SAFETY: `c"%s"` is a valid format string so `c_msg` is never interpreted as format specifiers.
        unsafe {
            libc::syslog(
                libc::LOG_DAEMON | libc::LOG_INFO,
                c"%s".as_ptr(),
                c_msg.as_ptr(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_fd_ignores_regular_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file = std::fs::File::create(&file_path).unwrap();
        use std::os::fd::AsRawFd;
        let fd = file.as_raw_fd();

        assert!(!notify_fd(fd));
        // SAFETY: `fd` is owned by `file`.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert!(flags >= 0);
    }

    #[test]
    fn test_notify_fd_signals_pipe() {
        let mut pipe_fds = [0; 2];
        // SAFETY: `pipe_fds` is a valid 2-element int array.
        let res = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
        assert_eq!(res, 0);

        let read_fd = pipe_fds[0];
        let write_fd = pipe_fds[1];

        assert!(notify_fd(write_fd));

        let mut buf = [0u8; 4];
        // SAFETY: `read_fd` is valid and `buf` is 4 bytes.
        let bytes_read = unsafe { libc::read(read_fd, buf.as_mut_ptr().cast(), 4) };
        assert_eq!(bytes_read, 1);
        assert_eq!(buf[0], b'\n');

        // SAFETY: `read_fd` is owned by this test.
        unsafe { libc::close(read_fd) };
    }
}
