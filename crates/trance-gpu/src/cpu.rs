// SPDX-License-Identifier: MIT

use crate::FilterMode;

/// Cached nearest-neighbor column map for stretch upscale.
pub struct StretchCache {
    src_w: u32,
    dst_w: u32,
    x_map: Vec<u32>,
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

    cache.ensure(src_w, dst_w);
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
pub fn upscale_stretch(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];
    let mut cache = StretchCache::new();
    upscale_stretch_into(&mut dst, src, src_w, src_h, dst_w, dst_h, &mut cache);
    dst
}

pub fn upscale_letterbox_into(
    dst: &mut [u8],
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    filter: FilterMode,
) {
    let needed = (dst_w * dst_h * 4) as usize;
    if dst.len() < needed {
        return;
    }
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        dst[..needed].fill(0);
        return;
    }

    dst[..needed].fill(0);

    let scale = (dst_w as f32 / src_w as f32).min(dst_h as f32 / src_h as f32);
    let display_w = (src_w as f32 * scale).floor() as u32;
    let display_h = (src_h as f32 * scale).floor() as u32;
    let offset_x = (dst_w - display_w) / 2;
    let offset_y = (dst_h - display_h) / 2;

    for dst_y in 0..display_h {
        for dst_x in 0..display_w {
            let out_x = offset_x + dst_x;
            let out_y = offset_y + dst_y;
            let color = sample_src(
                src,
                src_w,
                src_h,
                (dst_x as f32 + 0.5) / display_w as f32 * src_w as f32 - 0.5,
                (dst_y as f32 + 0.5) / display_h as f32 * src_h as f32 - 0.5,
                filter,
            );
            write_pixel(dst, dst_w, out_x, out_y, color);
        }
    }
}

pub fn upscale_letterbox(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    filter: FilterMode,
) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];
    upscale_letterbox_into(&mut dst, src, src_w, src_h, dst_w, dst_h, filter);
    dst
}

fn sample_src(
    src: &[u8],
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    filter: FilterMode,
) -> [u8; 4] {
    match filter {
        FilterMode::Nearest => sample_nearest(src, width, height, x, y),
        FilterMode::Linear => sample_bilinear(src, width, height, x, y),
    }
}

fn sample_nearest(src: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    let px = x.round().clamp(0.0, (width - 1) as f32) as u32;
    let py = y.round().clamp(0.0, (height - 1) as f32) as u32;
    read_pixel(src, width, px, py)
}

fn sample_bilinear(src: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    let x0 = x.floor().clamp(0.0, (width - 1) as f32) as u32;
    let y0 = y.floor().clamp(0.0, (height - 1) as f32) as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let c00 = read_pixel(src, width, x0, y0);
    let c10 = read_pixel(src, width, x1, y0);
    let c01 = read_pixel(src, width, x0, y1);
    let c11 = read_pixel(src, width, x1, y1);

    let mut out = [0u8; 4];
    for channel in 0..4 {
        let top = lerp(c00[channel] as f32, c10[channel] as f32, tx);
        let bottom = lerp(c01[channel] as f32, c11[channel] as f32, tx);
        out[channel] = lerp(top, bottom, ty).round() as u8;
    }
    out
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn read_pixel(src: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    if offset + 3 >= src.len() {
        return [0, 0, 0, 255];
    }
    [src[offset], src[offset + 1], src[offset + 2], src[offset + 3]]
}

fn write_pixel(dst: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4]) {
    let offset = ((y * width + x) * 4) as usize;
    if offset + 3 >= dst.len() {
        return;
    }
    dst[offset..offset + 4].copy_from_slice(&color);
}