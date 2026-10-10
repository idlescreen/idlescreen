// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Multi-protocol readiness notifications (systemd NOTIFY_SOCKET, fd 3 pipe, D-Bus).

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
        // Abstract socket address: sun_path starts with null byte
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as libc::sa_family_t;

        let bytes = abstract_path.as_bytes();
        let max_len = addr.sun_path.len() - 1; // 1 byte reserved for leading null
        let copy_len = bytes.len().min(max_len);

        addr.sun_path[0] = 0; // null byte indicates abstract socket
        for i in 0..copy_len {
            addr.sun_path[1 + i] = bytes[i] as libc::c_char;
        }

        let addr_len = (std::mem::size_of::<libc::sa_family_t>() + 1 + copy_len) as libc::socklen_t;
        use std::os::fd::AsRawFd;
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
        // Standard filesystem path
        sock.send_to(payload, Path::new(&notify_socket))?;
    }

    Ok(true)
}

/// Signals readiness via fd 3 (standard s6 / dinit readiness protocol).
///
/// If fd 3 is open, writes a newline `\n` to fd 3 and closes it.
pub fn notify_fd3() -> bool {
    let fd = 3;
    // Check if fd 3 is open
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return false;
    }

    let byte = *b"\n";
    let res = unsafe { libc::write(fd, byte.as_ptr().cast(), 1) };
    if res > 0 {
        unsafe { libc::close(fd) };
        true
    } else {
        false
    }
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
