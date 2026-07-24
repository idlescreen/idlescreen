// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Pixel sampling helpers for letterbox upscale.

use crate::FilterMode;

pub(super) fn sample_src(
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
    let x_clamped = x.clamp(0.0, (width - 1) as f32);
    let y_clamped = y.clamp(0.0, (height - 1) as f32);
    let x0 = x_clamped.floor() as u32;
    let y0 = y_clamped.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x_clamped - x0 as f32;
    let ty = y_clamped - y0 as f32;

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
    let offset = (y as usize * width as usize + x as usize) * 4;
    if offset + 3 >= src.len() {
        return [0, 0, 0, 255];
    }
    [
        src[offset],
        src[offset + 1],
        src[offset + 2],
        src[offset + 3],
    ]
}

pub(super) fn write_pixel(dst: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4]) {
    let offset = (y as usize * width as usize + x as usize) * 4;
    if offset + 3 >= dst.len() {
        return;
    }
    dst[offset..offset + 4].copy_from_slice(&color);
}
