// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Nearest-neighbor stretch upscale (fills destination, may distort aspect).

/// Cached nearest-neighbor column map for stretch upscale.
pub struct StretchCache {
    /// Source width last used to build `x_map` (test/cache introspection).
    pub src_w: u32,
    /// Destination width last used to build `x_map`.
    pub dst_w: u32,
    /// Nearest-neighbor source-x for each destination column.
    pub x_map: Vec<u32>,
}

impl StretchCache {
    pub fn new() -> Self {
        Self {
            src_w: 0,
            dst_w: 0,
            x_map: Vec::new(),
        }
    }

    pub fn ensure(&mut self, src_w: u32, dst_w: u32) {
        if self.src_w == src_w && self.dst_w == dst_w && self.x_map.len() == dst_w as usize {
            return;
        }
        self.src_w = src_w;
        self.dst_w = dst_w;
        self.x_map = (0..dst_w)
            .map(|dx| (dx as u64 * src_w as u64 / dst_w as u64) as u32)
            .collect();
    }
}

/// Fast integer nearest-neighbor stretch into `dst` (reuses `cache` x-map).
#[tracing::instrument(skip_all, fields(src_w, src_h, dst_w, dst_h))]
#[allow(clippy::too_many_arguments)]
pub fn upscale_stretch_into(
    dst: &mut [u8],
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    cache: &mut StretchCache,
) {
    let needed = (dst_w * dst_h * 4) as usize;
    if dst.len() < needed {
        return;
    }
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        dst[..needed].fill(0);
        return;
    }

    if src_w == dst_w && src_h == dst_h {
        let copy_len = needed.min(src.len());
        dst[..copy_len].copy_from_slice(&src[..copy_len]);
        if needed > copy_len {
            dst[copy_len..needed].fill(0);
        }
        return;
    }

    cache.ensure(src_w, dst_w);

    match (
        bytemuck::try_cast_slice::<u8, u32>(src),
        bytemuck::try_cast_slice_mut::<u8, u32>(&mut dst[..needed]),
    ) {
        (Ok(src_u32), Ok(dst_u32)) => {
            stretch_u32_rows(src_u32, dst_u32, src_w, src_h, dst_w, dst_h, cache)
        }
        _ => stretch_byte_rows(dst, src, src_w, src_h, dst_w, dst_h, needed, cache),
    }
}

fn stretch_u32_rows(
    src_u32: &[u32],
    dst_u32: &mut [u32],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    cache: &StretchCache,
) {
    for dy in 0..dst_h {
        let sy = (dy as u64 * src_h as u64 / dst_h as u64) as usize;
        let dst_row_start = dy as usize * dst_w as usize;
        let dst_row_end = dst_row_start + dst_w as usize;
        let src_row_start = sy * src_w as usize;

        if dst_row_end <= dst_u32.len() && src_row_start + src_w as usize <= src_u32.len() {
            let src_row_slice = &src_u32[src_row_start..src_row_start + src_w as usize];
            let dst_row_slice = &mut dst_u32[dst_row_start..dst_row_end];
            for (dx, val) in dst_row_slice.iter_mut().enumerate() {
                let sx = cache.x_map[dx] as usize;
                *val = src_row_slice[sx];
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stretch_byte_rows(
    dst: &mut [u8],
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    needed: usize,
    cache: &StretchCache,
) {
    // Fallback unaligned byte-copy path
    dst[..needed].fill(0);
    for dy in 0..dst_h {
        let sy = (dy as u64 * src_h as u64 / dst_h as u64) as u32;
        let src_row = sy as usize * src_w as usize * 4;
        let dst_row = dy as usize * dst_w as usize * 4;
        for dx in 0..dst_w as usize {
            let src_off = src_row + cache.x_map[dx] as usize * 4;
            let dst_off = dst_row + dx * 4;
            if src_off + 4 <= src.len() && dst_off + 4 <= dst.len() {
                dst[dst_off..dst_off + 4].copy_from_slice(&src[src_off..src_off + 4]);
            }
        }
    }
}

/// Fast integer nearest-neighbor stretch (allocates output).
#[allow(dead_code)]
pub fn upscale_stretch(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let needed = (dst_w as usize)
        .checked_mul(dst_h as usize)
        .and_then(|p| p.checked_mul(4))
        .unwrap_or(0);
    let mut dst = vec![0u8; needed];
    let mut cache = StretchCache::new();
    upscale_stretch_into(&mut dst, src, src_w, src_h, dst_w, dst_h, &mut cache);
    dst
}
