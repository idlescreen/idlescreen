// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use std::ffi::CString;
use std::ptr;

use crate::ffi_cell::{FfiTerminalCell, SHM_MAGIC, SharedMemoryHeader};
use crate::path_safety::is_valid_shm_name;

/// POSIX shared-memory region used for terminal-cell IPC.
///
/// Ownership: exclusive over `fd` + `mmap` mapping. `Drop` always `munmap`s,
/// `close`s, and (when `is_owner`) `shm_unlink`s. Named SHM only — memfd is not
/// used because the OOP runner re-opens by name.
pub struct SharedMemory {
    name: String,
    fd: libc::c_int,
    ptr: *mut libc::c_void,
    size: usize,
    is_owner: bool,
}

// SAFETY: `SharedMemory` is the exclusive owner of the fd and mapped pages.
// Moving across threads transfers that ownership; concurrent access to the
// mapping is a protocol concern (single writer per field), not a Send concern.
unsafe impl Send for SharedMemory {}

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
        // O_EXCL + 0600: refuse squatters and keep the object owner-private.
        // SAFETY: `c_name` is a valid CString; unlink best-effort for stale objects.
        unsafe {
            libc::shm_unlink(c_name.as_ptr());
        }
        // SAFETY: O_CREAT|O_EXCL|O_RDWR with mode 0600 on a validated name.
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

        // SAFETY: `fd` is open; size fits in off_t (capped at 64 MiB above).
        if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
            let err = std::io::Error::last_os_error();
            // SAFETY: clean up partially created object on size failure.
            unsafe {
                libc::close(fd);
                libc::shm_unlink(c_name.as_ptr());
            }
            return Err(format!("ftruncate failed: {err}"));
        }

        // SAFETY: MAP_SHARED over the full sized object; fail closed on MAP_FAILED.
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
            // SAFETY: release fd + name if mapping failed.
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

        // SAFETY: open existing named object; name validated above.
        let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0) };
        if fd < 0 {
            return Err(format!(
                "shm_open (open) failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        // SAFETY: MAP_SHARED; close fd on MAP_FAILED (non-owner does not unlink).
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
            // SAFETY: fd is open and owned by this path only.
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

    /// Mutable header view.
    ///
    /// # Safety
    /// - Mapping is live (`create`/`open` succeeded; `Drop` not yet run).
    /// - Caller serializes writers (daemon initializes; runner updates counter).
    /// - `self.size >= size_of::<SharedMemoryHeader>()` (enforced at open/create).
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn header_mut(&self) -> &mut SharedMemoryHeader {
        debug_assert!(!self.ptr.is_null() && self.ptr != libc::MAP_FAILED);
        debug_assert!(self.size >= std::mem::size_of::<SharedMemoryHeader>());
        // SAFETY: caller upholds mapping lifetime and single-writer protocol.
        unsafe { &mut *(self.ptr as *mut SharedMemoryHeader) }
    }

    /// Bounds-checked cell view. Rejects bad magic / dims that would exceed the map.
    ///
    /// # Safety
    /// Region must be mapped; length is validated against `self.size`. Concurrent
    /// mutation of header dims while this slice is live is undefined.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn cells_mut(&self) -> Result<&mut [FfiTerminalCell], String> {
        // SAFETY: same mapping invariants as `header_mut`.
        let header = unsafe { self.header_mut() };
        if header.magic != 0 && header.magic != SHM_MAGIC {
            return Err(format!(
                "shm header magic {:#x} != expected {:#x}",
                header.magic, SHM_MAGIC
            ));
        }
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
        // SAFETY: `needed <= self.size`; cells begin immediately after the header.
        let cells_ptr = unsafe { (self.ptr as *mut u8).add(header_sz) as *mut FfiTerminalCell };
        Ok(unsafe { std::slice::from_raw_parts_mut(cells_ptr, count) })
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        // SAFETY: reverse of create/open — unmap, close fd, unlink if we own the name.
        // Null/`MAP_FAILED` and fd < 0 guard against double-free after partial init.
        unsafe {
            if !self.ptr.is_null() && self.ptr != libc::MAP_FAILED {
                libc::munmap(self.ptr, self.size);
                self.ptr = ptr::null_mut();
            }
            if self.fd >= 0 {
                libc::close(self.fd);
                self.fd = -1;
            }
            if self.is_owner
                && let Ok(c_name) = CString::new(self.name.as_str())
            {
                libc::shm_unlink(c_name.as_ptr());
            }
        }
    }
}

#[cfg(test)]
#[path = "shm_tests.rs"]
mod tests;
