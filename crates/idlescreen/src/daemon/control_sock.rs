// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Direct Unix domain control socket for headless, systemd socket-activated, and non-systemd operation.

#![allow(unsafe_code)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub struct ControlListener {
    listener: UnixListener,
    path: PathBuf,
    owns_path: bool,
}

impl ControlListener {
    pub fn bind() -> std::io::Result<Self> {
        let path = resolve_control_sock_path()?;
        if let Some(inherited) = Self::from_systemd_listen_fds(path.clone()) {
            return Ok(inherited);
        }
        Self::bind_at_path(path)
    }

    pub fn from_systemd_listen_fds(path: PathBuf) -> Option<Self> {
        let parse_env = |k| std::env::var(k).ok()?.trim().parse::<u32>().ok();
        if parse_env("LISTEN_PID")? != std::process::id() || parse_env("LISTEN_FDS")? < 1 {
            return None;
        }
        Self::from_inherited_fd(3, path).ok()
    }

    pub fn from_inherited_fd(fd: RawFd, path: PathBuf) -> std::io::Result<Self> {
        // SAFETY: `fstat` inspects `fd` without taking ownership; we verify `S_IFSOCK` before wrapping.
        let is_sock = unsafe {
            let mut st: libc::stat = std::mem::zeroed();
            libc::fstat(fd, &raw mut st) == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFSOCK
        };
        if !is_sock {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "inherited FD is not a socket",
            ));
        }
        // SAFETY: `fd` was validated as an open socket descriptor passed for socket activation.
        let listener = unsafe { UnixListener::from_raw_fd(fd) };
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            path,
            owns_path: false,
        })
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
                    let mut resp = String::new();
                    if BufReader::new(&mut stream).read_line(&mut resp).is_ok()
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
        Ok(Self {
            listener,
            path,
            owns_path: true,
        })
    }

    #[rustfmt::skip]
    pub fn as_raw_fd(&self) -> RawFd { self.listener.as_raw_fd() }

    pub fn accept(&self) -> std::io::Result<UnixStream> {
        let (stream, _) = self.listener.accept()?;
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
        Ok(stream)
    }

    #[rustfmt::skip]
    pub fn path(&self) -> &std::path::Path { &self.path }
}

impl Drop for ControlListener {
    #[rustfmt::skip]
    fn drop(&mut self) { if self.owns_path { let _ = fs::remove_file(&self.path); } }
}

pub struct ControlClient;

impl ControlClient {
    pub fn send_command(cmd: &str) -> std::io::Result<String> {
        let mut stream = UnixStream::connect(resolve_control_sock_path()?)?;
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

#[rustfmt::skip]
pub fn resolve_control_sock_path() -> std::io::Result<PathBuf> { Ok(resolve_runtime_subdir().join("control.sock")) }

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn test_control_sock_ping_pong() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");
        let listener = ControlListener::bind_at_path(sock_path.clone()).unwrap();
        assert!(listener.path().exists());

        let mut client_stream = UnixStream::connect(&sock_path).unwrap();
        writeln!(client_stream, "PING").unwrap();
        let mut server_stream = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(&mut server_stream).read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "PING");
        writeln!(server_stream, "PONG").unwrap();

        let mut resp = String::new();
        BufReader::new(client_stream).read_line(&mut resp).unwrap();
        assert_eq!(resp.trim(), "PONG");
    }

    #[test]
    fn test_control_sock_refuses_when_active() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");
        let listener = ControlListener::bind_at_path(sock_path.clone()).unwrap();
        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok(mut stream) => {
                        let mut line = String::new();
                        if BufReader::new(&mut stream).read_line(&mut line).is_ok() && line.trim().eq_ignore_ascii_case("ping") {
                            let _ = writeln!(stream, "PONG");
                        }
                        return;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(2)),
                    Err(_) => return,
                }
            }
        });
        let second_bind = ControlListener::bind_at_path(sock_path);
        assert_eq!(second_bind.unwrap_err().kind(), std::io::ErrorKind::AddrInUse);
        let _ = handle.join();
    }

    #[test]
    fn test_control_sock_unlinks_stale_socket() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");
        drop(UnixListener::bind(&sock_path).unwrap());
        assert!(sock_path.exists());
        let new_listener = ControlListener::bind_at_path(sock_path).unwrap();
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
        assert_eq!(cfg.saver.active.as_deref(), Some("aurora"));
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
        let start = std::time::Instant::now();
        let mut reader = BufReader::new(listener.accept().unwrap().take(4096));
        let mut line = String::new();
        assert!(reader.read_line(&mut line).is_err());
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn test_inherited_fd_not_unlinked_on_drop() {
        use std::os::fd::IntoRawFd;
        let temp_dir = tempfile::tempdir().unwrap();
        let sock_path = temp_dir.path().join("control.sock");
        let raw_fd = UnixListener::bind(&sock_path).unwrap().into_raw_fd();
        {
            let inherited = ControlListener::from_inherited_fd(raw_fd, sock_path.clone()).unwrap();
            assert!(!inherited.owns_path);
        }
        assert!(sock_path.exists());
    }
}
