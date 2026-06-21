// SPDX-License-Identifier: MIT

use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::ptr;

use wayland_client::protocol::{wl_buffer, wl_shm};
use wayland_client::QueueHandle;

use super::state::SessionState;

pub struct MappedBuffer {
    pub wl_buffer: wl_buffer::WlBuffer,
    _memfd: OwnedFd,
    mapped_ptr: *mut u8,
    mapped_len: usize,
    width: u32,
    height: u32,
}

impl Drop for MappedBuffer {
    fn drop(&mut self) {
        if !self.mapped_ptr.is_null() && self.mapped_len > 0 {
            unsafe {
                libc::munmap(self.mapped_ptr.cast(), self.mapped_len);
            }
        }
    }
}

impl MappedBuffer {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn write_pixels(&mut self, pixels: &[u8]) -> bool {
        let stride = self.width.saturating_mul(4);
        let length = stride.saturating_mul(self.height) as usize;
        if pixels.len() < length || length > self.mapped_len {
            return false;
        }

        unsafe {
            let mapped = std::slice::from_raw_parts_mut(self.mapped_ptr, length);
            mapped.copy_from_slice(&pixels[..length]);
        }
        true
    }
}

pub fn create_solid_buffer(
    shm: &wl_shm::WlShm,
    queue: &QueueHandle<SessionState>,
    width: u32,
    height: u32,
    color: [u8; 3],
) -> Option<MappedBuffer> {
    let buffer = allocate_buffer(shm, queue, width, height)?;
    unsafe {
        let mapped = std::slice::from_raw_parts_mut(buffer.mapped_ptr, buffer.mapped_len);
        fill_argb8888(mapped, color);
    }
    Some(buffer)
}

pub fn ensure_frame_buffer(
    existing: &mut Option<MappedBuffer>,
    shm: &wl_shm::WlShm,
    queue: &QueueHandle<SessionState>,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> bool {
    if width == 0 || height == 0 {
        return false;
    }

    let needs_new = existing.as_ref().is_none_or(|buffer| {
        buffer.width != width || buffer.height != height
    });

    if needs_new {
        *existing = allocate_buffer(shm, queue, width, height);
    }

    let Some(buffer) = existing.as_mut() else {
        return false;
    };
    if buffer.write_pixels(pixels) {
        return true;
    }

    *existing = allocate_buffer(shm, queue, width, height);
    existing
        .as_mut()
        .is_some_and(|buffer| buffer.write_pixels(pixels))
}

fn allocate_buffer(
    shm: &wl_shm::WlShm,
    queue: &QueueHandle<SessionState>,
    width: u32,
    height: u32,
) -> Option<MappedBuffer> {
    if width == 0 || height == 0 {
        return None;
    }

    let stride = width.saturating_mul(4);
    let length = stride.saturating_mul(height) as usize;
    let memfd = create_memfd(length)?;

    let mapped_ptr = unsafe {
        let address = libc::mmap(
            ptr::null_mut(),
            length,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            memfd.as_fd().as_raw_fd(),
            0,
        );
        if address == libc::MAP_FAILED {
            return None;
        }
        address as *mut u8
    };

    let pool = shm.create_pool(memfd.as_fd(), length as i32, queue, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        wl_shm::Format::Argb8888,
        queue,
        (),
    );

    Some(MappedBuffer {
        wl_buffer: buffer,
        _memfd: memfd,
        mapped_ptr,
        mapped_len: length,
        width,
        height,
    })
}

fn create_memfd(length: usize) -> Option<OwnedFd> {
    let fd = unsafe { libc::memfd_create(c"trance-overlay".as_ptr(), 0) };
    if fd < 0 {
        return None;
    }

    let owned = unsafe { OwnedFd::from_raw_fd(fd) };
    if unsafe { libc::ftruncate(owned.as_fd().as_raw_fd(), length as i64) } != 0 {
        return None;
    }

    Some(owned)
}

fn fill_argb8888(pixels: &mut [u8], color: [u8; 3]) {
    let mut offset = 0;
    while offset + 3 < pixels.len() {
        pixels[offset] = color[2];
        pixels[offset + 1] = color[1];
        pixels[offset + 2] = color[0];
        pixels[offset + 3] = 0xFF;
        offset += 4;
    }
}