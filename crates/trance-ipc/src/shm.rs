// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

use std::ffi::CString;
use std::ptr;

use crate::ffi_cell::{FfiTerminalCell, SharedMemoryHeader};

/// POSIX SHM object names we create look like `/trance-shm-<pid>-<idx>`.
/// Reject anything else so a compromised arg vector cannot open arbitrary objects.
pub fn is_valid_shm_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("/trance-shm-") else {
        return false;
    };
    if rest.is_empty() || rest.len() > 64 {
        return false;
    }
    rest.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// UDS control paths must be absolute, end in `.sock`, stay under path limits,
/// and must not contain nulls or `..` segments.
pub fn is_plausible_socket_path(path: &str) -> bool {
    if path.is_empty() || path.len() >= 108 {
        return false;
    }
    if path.contains('\0') || !path.starts_with('/') || !path.ends_with(".sock") {
        return false;
    }
    if path.split('/').any(|seg| seg == "..") {
        return false;
    }
    true
}

pub struct SharedMemory {
    name: String,
    fd: libc::c_int,
    ptr: *mut libc::c_void,
    size: usize,
    is_owner: bool,
    is_memfd: bool,
}

impl SharedMemory {
    pub fn create(name: &str, size: usize) -> Result<Self, String> {
        if !is_valid_shm_name(name) {
            return Err(format!("invalid shm name: {name}"));
        }
        if size < std::mem::size_of::<SharedMemoryHeader>() || size > 64 * 1024 * 1024 {
            return Err(format!("shm size out of range: {size}"));
        }
        let c_name = CString::new(name).map_err(|e| e.to_string())?;

        // Named POSIX SHM only: the IPC child re-opens by name (`SharedMemory::open`).
        // memfd would be anonymous and unopenable by the runner process.
        // O_EXCL + 0600: refuse squatters and keep the object owner-private.
        unsafe {
            libc::shm_unlink(c_name.as_ptr());
        }
        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                0o600,
            )
        };
        if fd < 0 {
            return Err(format!(
                "shm_open (create) failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
                libc::shm_unlink(c_name.as_ptr());
            }
            return Err(format!("ftruncate failed: {err}"));
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
                libc::shm_unlink(c_name.as_ptr());
            }
            return Err(format!("mmap failed: {err}"));
        }

        Ok(Self {
            name: name.to_string(),
            fd,
            ptr,
            size,
            is_owner: true,
            is_memfd: false,
        })
    }

    pub fn open(name: &str, size: usize) -> Result<Self, String> {
        if !is_valid_shm_name(name) {
            return Err(format!("invalid shm name: {name}"));
        }
        if size < std::mem::size_of::<SharedMemoryHeader>() || size > 64 * 1024 * 1024 {
            return Err(format!("shm size out of range: {size}"));
        }
        let c_name = CString::new(name).map_err(|e| e.to_string())?;

        let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0) };
        if fd < 0 {
            return Err(format!(
                "shm_open (open) failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(format!("mmap failed: {err}"));
        }

        Ok(Self {
            name: name.to_string(),
            fd,
            ptr,
            size,
            is_owner: false,
            is_memfd: false,
        })
    }

    pub fn fd(&self) -> libc::c_int {
        self.fd
    }

    pub fn ptr(&self) -> *mut libc::c_void {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// # Safety
    /// Caller must ensure shared memory region is validly mapped and non-null.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn header_mut(&self) -> &mut SharedMemoryHeader {
        unsafe { &mut *(self.ptr as *mut SharedMemoryHeader) }
    }

    /// Bounds-checked cell view. Rejects header dims that would exceed the map.
    ///
    /// # Safety
    /// Region must be mapped; only the *length* is validated against `self.size`.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn cells_mut(&self) -> Result<&mut [FfiTerminalCell], String> {
        let header = unsafe { self.header_mut() };
        let cols = header.cols as usize;
        let rows = header.rows as usize;
        let count = cols
            .checked_mul(rows)
            .ok_or_else(|| "shm header cell count overflow".to_string())?;
        let header_sz = std::mem::size_of::<SharedMemoryHeader>();
        let cell_sz = std::mem::size_of::<FfiTerminalCell>();
        let needed = header_sz
            .checked_add(
                count
                    .checked_mul(cell_sz)
                    .ok_or_else(|| "shm cell byte count overflow".to_string())?,
            )
            .ok_or_else(|| "shm size overflow".to_string())?;
        if needed > self.size {
            return Err(format!(
                "shm header dims {cols}x{rows} need {needed} bytes, map is {}",
                self.size
            ));
        }
        let cells_ptr = unsafe { (self.ptr as *mut u8).add(header_sz) as *mut FfiTerminalCell };
        Ok(unsafe { std::slice::from_raw_parts_mut(cells_ptr, count) })
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() && self.ptr != libc::MAP_FAILED {
                libc::munmap(self.ptr, self.size);
            }
            if self.fd >= 0 {
                libc::close(self.fd);
            }
            if self.is_owner
                && !self.is_memfd
                && let Ok(c_name) = CString::new(self.name.clone())
            {
                libc::shm_unlink(c_name.as_ptr());
            }
        }
    }
}

#[cfg(test)]
#[path = "shm_tests.rs"]
mod tests;
