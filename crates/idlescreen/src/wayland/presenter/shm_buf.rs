// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared-memory (`wl_shm`) buffer allocation with checked dimensions,
//! compositor release tracking, and non-blocking Wayland socket polling.

#![allow(unsafe_code)]

use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use wayland_client::protocol::{wl_buffer, wl_shm};
use wayland_client::{Connection, QueueHandle};

use super::wl_state::WaylandState;

pub struct MappedBuffer {
    pub wl_buffer: wl_buffer::WlBuffer,
    _memfd: OwnedFd,
    mapped_ptr: *mut u32,
    pub width: u32,
    pub height: u32,
    pub is_10bit_hdr: bool,
    pub busy: Arc<AtomicBool>,
}

impl Drop for MappedBuffer {
    fn drop(&mut self) {
        self.wl_buffer.destroy();
        if !self.mapped_ptr.is_null()
            && let Some(pixels) = (self.width as usize).checked_mul(self.height as usize)
            && let Some(length) = pixels.checked_mul(4)
            && length > 0
        {
            // SAFETY: `mapped_ptr` was returned by `libc::mmap` with `length` bytes and has not been unmapped.
            unsafe {
                libc::munmap(self.mapped_ptr.cast::<libc::c_void>(), length);
            }
        }
    }
}

impl MappedBuffer {
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        let len = (self.width as usize).saturating_mul(self.height as usize);
        if self.mapped_ptr.is_null() || len == 0 {
            return &mut [];
        }
        // SAFETY: `mapped_ptr` points to a valid `mmap` region of `len * 4` bytes aligned for `u32`.
        unsafe { std::slice::from_raw_parts_mut(self.mapped_ptr, len) }
    }
}

pub fn poll_and_read_events<D>(
    connection: &Connection,
    event_queue: &mut wayland_client::EventQueue<D>,
    state: &mut D,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    connection.flush()?;

    if let Some(guard) = connection.prepare_read() {
        let fd = guard.connection_fd().as_raw_fd();
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };
        let timeout_ms = i32::try_from(timeout.as_millis().min(50)).unwrap_or(50);
        // SAFETY: `pfd` is a valid stack `pollfd` struct with count 1.
        let ret = unsafe { libc::poll(&raw mut pfd, 1, timeout_ms) };
        if ret > 0 && (pfd.revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP)) != 0 {
            let _ = guard.read();
        }
    }

    event_queue.dispatch_pending(state)?;
    Ok(())
}

pub fn allocate_shm_buffer(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
    width: u32,
    height: u32,
) -> Option<MappedBuffer> {
    allocate_shm_buffer_with_format(shm, qh, width, height, wl_shm::Format::Argb8888, false)
}

pub fn allocate_shm_buffer_with_format(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
    width: u32,
    height: u32,
    format: wl_shm::Format,
    is_10bit_hdr: bool,
) -> Option<MappedBuffer> {
    if width == 0 || height == 0 {
        return None;
    }
    let stride = width.checked_mul(4)?;
    let length = (stride as usize).checked_mul(height as usize)?;
    let width_i32 = i32::try_from(width).ok()?;
    let height_i32 = i32::try_from(height).ok()?;
    let stride_i32 = i32::try_from(stride).ok()?;
    let length_i32 = i32::try_from(length).ok()?;
    let length_i64 = i64::try_from(length).ok()?;

    // SAFETY: `c"idlescreen-shm"` is a valid null-terminated C string literal.
    let fd = unsafe { libc::memfd_create(c"idlescreen-shm".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return None;
    }
    // SAFETY: `fd` is a newly created file descriptor owned exclusively by this function.
    let memfd = unsafe { OwnedFd::from_raw_fd(fd) };
    // SAFETY: `memfd` is a valid open file descriptor and `length_i64` is positive.
    if unsafe { libc::ftruncate(memfd.as_fd().as_raw_fd(), length_i64) } != 0 {
        return None;
    }
    // SAFETY: mapping `length` bytes of the newly sized `memfd` with read/write shared access.
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            length,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            memfd.as_fd().as_raw_fd(),
            0,
        )
    };
    if ptr == libc::MAP_FAILED || ptr.is_null() {
        return None;
    }

    let busy = Arc::new(AtomicBool::new(false));
    let pool = shm.create_pool(memfd.as_fd(), length_i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        width_i32,
        height_i32,
        stride_i32,
        format,
        qh,
        busy.clone(),
    );
    pool.destroy();

    Some(MappedBuffer {
        wl_buffer: buffer,
        _memfd: memfd,
        mapped_ptr: ptr.cast::<u32>(),
        width,
        height,
        is_10bit_hdr,
        busy,
    })
}
