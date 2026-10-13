// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Linux kernel flock PID file management.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;

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
        }

        // Loop up to 5 times to guard against races where the file was unlinked between open and flock
        for _ in 0..5 {
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

            // Verify that the file we locked still has the same device and inode as the file at path
            let mut fd_stat: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe { libc::fstat(raw_fd, &raw mut fd_stat) } < 0 {
                return Err(std::io::Error::last_os_error());
            }

            if let Ok(path_meta) = fs::metadata(&path) {
                use std::os::unix::fs::MetadataExt;
                if fd_stat.st_dev as u64 != path_meta.dev()
                    || fd_stat.st_ino as u64 != path_meta.ino()
                {
                    // File was replaced or unlinked before we locked it; retry opening current path
                    continue;
                }
            } else {
                // File was unlinked, retry creation
                continue;
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
        // Only unlink if the file at self.path still matches our locked file's device and inode
        let mut fd_stat: libc::stat = unsafe { std::mem::zeroed() };
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
        assert_eq!(lock2.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        drop(lock1);
        assert!(!is_daemon_running(&path));

        // After drop, should succeed again
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

        // Replace the file at path with a different file (different inode)
        fs::remove_file(&path).unwrap();
        fs::write(&path, "12345").unwrap();

        // Dropping the original lock should NOT remove the foreign file
        drop(lock);
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "12345");
    }
}
