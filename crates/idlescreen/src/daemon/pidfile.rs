// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Linux kernel flock PID file management.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;

pub struct PidFile {
    _fd: OwnedFd,
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
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        let raw_fd = file.as_raw_fd();
        let res = unsafe { libc::flock(raw_fd, libc::LOCK_EX | libc::LOCK_NB) };
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Successfully locked! Truncate and write current PID without releasing lock
        file.set_len(0)?;
        use std::io::Seek;
        file.rewind()?;
        let pid = std::process::id();
        writeln!(file, "{pid}")?;
        file.flush()?;

        use std::os::fd::IntoRawFd;
        let owned_fd = unsafe { OwnedFd::from_raw_fd(file.into_raw_fd()) };

        Ok(Self {
            _fd: owned_fd,
            path,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Checks whether an instance is currently holding an exclusive kernel lock on `path`.
pub fn is_daemon_running(path: &std::path::Path) -> bool {
    let Ok(file) = OpenOptions::new().read(true).write(true).open(path) else {
        return false;
    };
    let res = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if res < 0 {
        let err = std::io::Error::last_os_error();
        err.kind() == std::io::ErrorKind::WouldBlock
    } else {
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        false
    }
}

pub fn resolve_pid_path() -> std::io::Result<PathBuf> {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR")
        && !runtime_dir.trim().is_empty()
    {
        return Ok(PathBuf::from(runtime_dir).join("idlescreen/idlescreen.pid"));
    }

    let uid = unsafe { libc::getuid() };
    Ok(PathBuf::from(format!(
        "/tmp/idlescreen-{uid}/idlescreen.pid"
    )))
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

        // Second acquire should fail with WouldBlock
        let lock2 = PidFile::acquire_at_path(path.clone());
        assert!(lock2.is_err());

        drop(lock1);
        assert!(!is_daemon_running(&path));

        // After drop, should succeed again
        let lock3 =
            PidFile::acquire_at_path(path.clone()).expect("acquire after drop must succeed");
        assert!(is_daemon_running(&path));
        drop(lock3);
        assert!(!is_daemon_running(&path));
    }
}
