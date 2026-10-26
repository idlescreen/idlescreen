// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Buffer pool rotation and retirement.
//!
//! A `wl_buffer` may only be destroyed — and its shm mapping only
//! unmapped — after the compositor has sent `wl_buffer.release`. A buffer
//! that is still attached to a committed surface is therefore *retired*:
//! it stays alive until its release arrives, and is only then dropped.

use std::sync::atomic::Ordering;

use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use crate::wayland::presenter::shm_buf::{MappedBuffer, allocate_shm_buffer_with_format};
use crate::wayland::presenter::wl_state::WaylandState;

/// Upper bound on retired (still compositor-held) buffers per surface. Bounds
/// memory when the compositor lags: frames are dropped rather than freeing a
/// buffer the compositor may still be reading.
pub const MAX_RETIRED_BUFFERS: usize = 8;

/// Reclaims retired buffers whose release has arrived.
///
/// Returns true when a released buffer was freed.
pub fn reclaim_released(retired: &mut Vec<MappedBuffer>) -> bool {
    let before = retired.len();
    retired.retain(|b| b.busy.load(Ordering::Acquire));
    before != retired.len()
}

/// Geometry and pixel format a frame buffer must match.
pub struct BufferTarget {
    pub width: u32,
    pub height: u32,
    pub format: wl_shm::Format,
    pub is_10bit: bool,
}

/// Picks the buffer to draw the next frame into and makes sure it matches
/// the requested size.
///
/// Returns false when no suitable buffer is available (the frame should be
/// dropped rather than free a buffer the compositor may still read).
pub fn select_buffer(
    bufs: &mut [MappedBuffer; 2],
    retired: &mut Vec<MappedBuffer>,
    current: &mut usize,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandState>,
    target: &BufferTarget,
) -> bool {
    let BufferTarget {
        width: target_w,
        height: target_h,
        format: fmt,
        is_10bit,
    } = *target;
    if bufs[*current].busy.load(Ordering::Acquire) {
        if !bufs[*current ^ 1].busy.load(Ordering::Acquire) {
            *current ^= 1;
        } else if retired.len() >= MAX_RETIRED_BUFFERS {
            // Both buffers are still held by the compositor and the retire
            // list is saturated: drop this frame rather than free memory the
            // compositor may still be reading.
            return false;
        } else if let Some(b) =
            allocate_shm_buffer_with_format(shm, qh, target_w, target_h, fmt, is_10bit)
        {
            retire(bufs, retired, current, b);
        }
    }
    if (target_w != bufs[*current].width || target_h != bufs[*current].height)
        && let Some(b) = allocate_shm_buffer_with_format(shm, qh, target_w, target_h, fmt, is_10bit)
    {
        retire(bufs, retired, current, b);
    }
    true
}

/// Replaces the current buffer, retiring the old one when the compositor
/// still holds it.
fn retire(
    bufs: &mut [MappedBuffer; 2],
    retired: &mut Vec<MappedBuffer>,
    current: &mut usize,
    new: MappedBuffer,
) {
    let old = std::mem::replace(&mut bufs[*current], new);
    if old.busy.load(Ordering::Acquire) {
        retired.push(old);
    }
}
