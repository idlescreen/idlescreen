// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Socketpair and memfd shared-memory setup for idlescreen-extras IPC.

#![allow(unsafe_code)]

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;

use crate::cell::TerminalCell;

pub const MAX_CELLS: usize = 512 * 256;

pub struct ShmMapping {
    pub ptr: *mut TerminalCell,
    pub len: usize,
}

impl Drop for ShmMapping {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.len > 0 {
            // SAFETY: `self.ptr` was mapped with `mmap` of length `self.len`.
            unsafe {
                libc::munmap(self.ptr.cast(), self.len);
            }
        }
    }
}

pub fn create_socketpair() -> std::io::Result<(UnixStream, OwnedFd)> {
    let mut fds = [0 as RawFd; 2];
    // SAFETY: `fds` is a valid 2-element `RawFd` array for `socketpair`.
    let res = unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
            0,
            fds.as_mut_ptr(),
        )
    };
    if res < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: Both `fds[0]` and `fds[1]` are freshly created valid file descriptors.
    unsafe {
        Ok((
            UnixStream::from_raw_fd(fds[0]),
            OwnedFd::from_raw_fd(fds[1]),
        ))
    }
}

pub fn create_sealed_shm() -> std::io::Result<(OwnedFd, ShmMapping)> {
    let memfd_name = c"idlescreen-extras-shm";
    // SAFETY: `memfd_name` is a valid null-terminated CStr.
    let raw_shm = unsafe {
        libc::syscall(
            libc::SYS_memfd_create,
            memfd_name.as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        ) as RawFd
    };
    if raw_shm < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `raw_shm` is a valid open memfd descriptor.
    let shm_fd = unsafe { OwnedFd::from_raw_fd(raw_shm) };
    let shm_len = MAX_CELLS * std::mem::size_of::<TerminalCell>();
    // SAFETY: `shm_fd` is a valid memfd descriptor.
    if unsafe { libc::ftruncate(shm_fd.as_raw_fd(), shm_len as libc::off_t) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let seals = libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_SEAL;
    // SAFETY: `shm_fd` was created with `MFD_ALLOW_SEALING`.
    if unsafe { libc::fcntl(shm_fd.as_raw_fd(), libc::F_ADD_SEALS, seals) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `shm_fd` has size `shm_len` and can be shared-mapped RW.
    let shm_ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            shm_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            shm_fd.as_raw_fd(),
            0,
        )
    };
    if shm_ptr == libc::MAP_FAILED {
        return Err(std::io::Error::other(
            "failed to mmap shared memory in parent",
        ));
    }
    Ok((
        shm_fd,
        ShmMapping {
            ptr: shm_ptr.cast(),
            len: shm_len,
        },
    ))
}
