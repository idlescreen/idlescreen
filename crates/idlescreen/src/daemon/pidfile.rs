// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Linux kernel flock PID file management.

#![allow(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

use super::control_sock::resolve_runtime_subdir;

#[derive(Debug)]
pub struct PidFile {
    fd: OwnedFd,
    path: PathBuf,
}

impl PidFile {
    /// Attempts to acquire an exclusive, non-blocking kernel lock on the pid file.
    ///
    /// If another instance holds the lock, returns Err(ErrorKind::WouldBlock).
    pub fn acquire() -> std::io::Result<Self> {
        Self::acquire_at_path(resolve_pid_path()?)
    }

    pub fn acquire_at_path(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }

        for _ in 0..5 {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                // Refuse to follow a symlink: the pidfile path is inside the
                // user's runtime dir and must never be written through a
                // link planted by another process.
                .custom_flags(libc::O_NOFOLLOW)
                .open(&path)?;

            let raw_fd = file.as_raw_fd();
            // SAFETY: `raw_fd` is a valid open file descriptor owned by `file`.
            let res = unsafe { libc::flock(raw_fd, libc::LOCK_EX | libc::LOCK_NB) };
            if res < 0 {
                return Err(std::io::Error::last_os_error());
            }

            // SAFETY: `libc::stat` is a plain C struct safe to zero-initialize.
            let mut fd_stat: libc::stat = unsafe { std::mem::zeroed() };
            // SAFETY: `raw_fd` is valid and `fd_stat` is writable.
            if unsafe { libc::fstat(raw_fd, &raw mut fd_stat) } < 0 {
                return Err(std::io::Error::last_os_error());
            }

            if let Ok(path_meta) = fs::metadata(&path) {
                use std::os::unix::fs::MetadataExt;
                if fd_stat.st_dev as u64 != path_meta.dev()
                    || fd_stat.st_ino as u64 != path_meta.ino()
                {
                    continue;
                }
            } else {
                continue;
            }

            file.set_len(0)?;
            use std::io::Seek;
            file.rewind()?;
            let pid = std::process::id();
            writeln!(file, "{pid}")?;
            file.flush()?;

            use std::os::fd::IntoRawFd;
            // SAFETY: `file.into_raw_fd()` transfers ownership of the valid open descriptor.
            let owned_fd = unsafe { OwnedFd::from_raw_fd(file.into_raw_fd()) };

            return Ok(Self { fd: owned_fd, path });
        }

        Err(std::io::Error::other(
            "Failed to acquire stable PID file lock after retries",
        ))
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        // SAFETY: `libc::stat` is safe to zero-initialize.
        let mut fd_stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: `self.fd` is a valid `OwnedFd`.
        if unsafe { libc::fstat(self.fd.as_raw_fd(), &raw mut fd_stat) } == 0
            && let Ok(path_meta) = fs::metadata(&self.path)
        {
            use std::os::unix::fs::MetadataExt;
            if fd_stat.st_dev as u64 == path_meta.dev() && fd_stat.st_ino as u64 == path_meta.ino()
            {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

/// Checks whether an instance is currently holding an exclusive kernel lock on `path`.
pub fn is_daemon_running(path: &std::path::Path) -> bool {
    let Ok(file) = OpenOptions::new().read(true).write(true).open(path) else {
        return false;
    };
    // SAFETY: `file` is a valid open file.
    let res = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if res < 0 {
        let err = std::io::Error::last_os_error();
        err.kind() == std::io::ErrorKind::WouldBlock
    } else {
        // SAFETY: Releases the probe lock immediately before closing `file`.
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        false
    }
}

pub fn resolve_pid_path() -> std::io::Result<PathBuf> {
    Ok(resolve_runtime_subdir().join("idlescreen.pid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_file_locking() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("idlescreen.pid");

        let lock1 = PidFile::acquire_at_path(path.clone()).expect("first acquire must succeed");
        assert!(lock1.path().exists());
        assert!(is_daemon_running(&path));

        let lock2 = PidFile::acquire_at_path(path.clone());
        assert!(lock2.is_err());
        assert_eq!(lock2.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        drop(lock1);
        assert!(!is_daemon_running(&path));

        let lock3 =
            PidFile::acquire_at_path(path.clone()).expect("acquire after drop must succeed");
        assert!(is_daemon_running(&path));
        drop(lock3);
        assert!(!is_daemon_running(&path));
    }

    #[test]
    fn test_pid_file_safe_drop_ignores_foreign_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("idlescreen.pid");

        let lock = PidFile::acquire_at_path(path.clone()).expect("acquire must succeed");
        assert!(path.exists());

        fs::remove_file(&path).unwrap();
        fs::write(&path, "12345").unwrap();

        drop(lock);
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "12345");
    }
}
