// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct Unix domain control socket for headless and non-systemd operation.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

pub struct ControlListener {
    listener: UnixListener,
    path: PathBuf,
}

impl ControlListener {
    pub fn bind() -> std::io::Result<Self> {
        Self::bind_at_path(resolve_control_sock_path()?)
    }

    pub fn bind_at_path(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Remove stale socket if present
        let _ = fs::remove_file(&path);

        let listener = UnixListener::bind(&path)?;

        // Set strict permissions (0600)
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));

        // Set non-blocking for epoll integration
        listener.set_nonblocking(true)?;

        Ok(Self { listener, path })
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }

    pub fn accept(&self) -> std::io::Result<UnixStream> {
        let (stream, _) = self.listener.accept()?;
        Ok(stream)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for ControlListener {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub struct ControlClient;

impl ControlClient {
    pub fn send_command(cmd: &str) -> std::io::Result<String> {
        let path = resolve_control_sock_path()?;
        let mut stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;

        writeln!(stream, "{cmd}")?;
        stream.flush()?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(response.trim().to_string())
    }
}

pub fn resolve_control_sock_path() -> std::io::Result<PathBuf> {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR")
        && !runtime_dir.trim().is_empty()
    {
        return Ok(PathBuf::from(runtime_dir).join("idlescreen/control.sock"));
    }

    let uid = unsafe { libc::getuid() };
    Ok(PathBuf::from(format!("/tmp/idlescreen-{uid}/control.sock")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_sock_ping_pong() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");

        let listener = ControlListener::bind_at_path(sock_path.clone()).expect("bind must succeed");
        assert!(listener.path().exists());

        // Connect client
        let mut client_stream = UnixStream::connect(&sock_path).expect("connect must succeed");
        writeln!(client_stream, "PING").unwrap();

        let mut server_stream = listener.accept().expect("accept must succeed");
        let mut reader = BufReader::new(&mut server_stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "PING");

        writeln!(server_stream, "PONG").unwrap();

        let mut client_reader = BufReader::new(client_stream);
        let mut resp = String::new();
        client_reader.read_line(&mut resp).unwrap();
        assert_eq!(resp.trim(), "PONG");
    }
}
