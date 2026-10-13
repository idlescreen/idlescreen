// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct Unix domain control socket for headless and non-systemd operation.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

#[derive(Debug)]
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
        if path.exists() {
            // Check if another daemon is actively listening and responsive
            if let Ok(mut stream) = UnixStream::connect(&path) {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(150)));
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(150)));
                if writeln!(stream, "ping").is_ok() {
                    let mut reader = BufReader::new(&mut stream);
                    let mut resp = String::new();
                    if reader.read_line(&mut resp).is_ok()
                        && resp.trim().eq_ignore_ascii_case("pong")
                    {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::AddrInUse,
                            "An active idlescreen daemon is already responding on control socket",
                        ));
                    }
                }
            }
            // Socket exists but is stale or not responding
            let _ = fs::remove_file(&path);
        }

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

    #[test]
    fn test_control_sock_refuses_when_active() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");

        let listener =
            ControlListener::bind_at_path(sock_path.clone()).expect("first bind must succeed");

        // Spawn responsive thread answering ping. The listener is non-blocking
        // (it is driven by epoll in the daemon), so retry accept until the
        // client connects instead of racing it.
        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok(mut stream) => {
                        let mut reader = BufReader::new(&mut stream);
                        let mut line = String::new();
                        if reader.read_line(&mut line).is_ok()
                            && line.trim().eq_ignore_ascii_case("ping")
                        {
                            let _ = writeln!(stream, "PONG");
                        }
                        return;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                    Err(_) => return,
                }
            }
        });

        // Second bind attempt should fail with AddrInUse because active daemon responded
        let second_bind = ControlListener::bind_at_path(sock_path.clone());
        assert!(second_bind.is_err());
        assert_eq!(
            second_bind.unwrap_err().kind(),
            std::io::ErrorKind::AddrInUse
        );

        let _ = handle.join();
    }

    #[test]
    fn test_control_sock_unlinks_stale_socket() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");

        // Create an inactive/dead socket
        {
            let _listener = UnixListener::bind(&sock_path).unwrap();
            // drops _listener, but file remains on disk in disconnected state
        }
        assert!(sock_path.exists());

        // bind_at_path should recognize the stale socket and rebind cleanly
        let new_listener =
            ControlListener::bind_at_path(sock_path.clone()).expect("stale socket must be rebound");
        assert!(new_listener.path().exists());
    }
}
