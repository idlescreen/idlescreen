// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct Unix domain control socket for headless and non-systemd operation.

#![allow(unsafe_code)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

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
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
        if path.exists() {
            if let Ok(mut stream) = UnixStream::connect(&path) {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
                let _ = stream.set_write_timeout(Some(Duration::from_millis(150)));
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
            let _ = fs::remove_file(&path);
        }

        let listener = UnixListener::bind(&path)?;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        listener.set_nonblocking(true)?;

        Ok(Self { listener, path })
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.listener.as_raw_fd()
    }

    pub fn accept(&self) -> std::io::Result<UnixStream> {
        let (stream, _) = self.listener.accept()?;
        // Ensure accepted client stream has a strict 100ms timeout so an unprivileged client
        // connecting without sending a newline cannot stall the single-threaded reactor (B5).
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
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
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;

        writeln!(stream, "{cmd}")?;
        stream.flush()?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(response.trim().to_string())
    }
}

pub fn resolve_runtime_subdir() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR")
        && !runtime_dir.trim().is_empty()
    {
        return PathBuf::from(runtime_dir).join("idlescreen");
    }

    // SAFETY: `getuid()` is always thread-safe and cannot fail on POSIX.
    let uid = unsafe { libc::getuid() };
    let run_user = PathBuf::from(format!("/run/user/{uid}"));
    if let Ok(meta) = fs::metadata(&run_user)
        && meta.is_dir()
        && meta.uid() == uid
    {
        return run_user.join("idlescreen");
    }

    PathBuf::from(format!("/tmp/idlescreen-{uid}"))
}

pub fn resolve_control_sock_path() -> std::io::Result<PathBuf> {
    Ok(resolve_runtime_subdir().join("control.sock"))
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

        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
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
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => return,
                }
            }
        });

        let second_bind = ControlListener::bind_at_path(sock_path);
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
        {
            let _listener = UnixListener::bind(&sock_path).unwrap();
        }
        assert!(sock_path.exists());
        let new_listener =
            ControlListener::bind_at_path(sock_path).expect("stale socket must be rebound");
        assert!(new_listener.path().exists());
    }

    #[test]
    fn test_yaml_to_toml_migration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let yaml_path = temp_dir.path().join("config.yaml");
        let toml_path = temp_dir.path().join("config.toml");
        let yaml_src = "general:\n  idle_timeout_secs: 180 # comment\n  primary_output: \"DP-1\" # inline comment\nsaver:\n  active: 'aurora' # single quote\n  ascii:\n    mode: \"os\"\n    targets: [\"os\", 'de'] # flow targets\n    text: \"MyFedora\"\n    fps: 30\nextra:\n  mode: \"ignored\"\n";
        fs::write(&yaml_path, yaml_src).unwrap();
        let cfg = super::super::config::migrate_yaml_to_toml(&yaml_path, &toml_path).unwrap();
        assert_eq!(cfg.general.idle_timeout_secs, 180);
        assert_eq!(cfg.general.primary_output, "DP-1");
        assert_eq!(cfg.saver.active.as_deref(), Some("aurora"));
        assert_eq!(cfg.saver.ascii.mode, "os");
        assert_eq!(cfg.saver.ascii.targets, vec!["os", "de"]);
        assert!(toml_path.exists());
    }

    #[test]
    fn test_control_sock_partial_write_timeout_prevents_dos() {
        use std::io::Read;
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");
        let listener = ControlListener::bind_at_path(sock_path.clone()).unwrap();

        let mut slow_client = UnixStream::connect(&sock_path).unwrap();
        slow_client.write_all(b"partial_without_newline").unwrap();
        slow_client.flush().unwrap();

        let start = std::time::Instant::now();
        let server_stream = listener.accept().unwrap();
        let mut reader = BufReader::new(server_stream.take(4096));
        let mut line = String::new();
        let res = reader.read_line(&mut line);
        assert!(res.is_err(), "partial write without newline must time out");
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "read timeout must bound blocking to ~100ms"
        );
    }
}
