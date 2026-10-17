// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 5-act SNES Mega Man X pixel choreography: Teleport In, Level-3 X-Buster
//! Charge, Spiral Crush Blast + SUPERFEDORA Explosions, and Teleport Out.

use super::mmx_data::{MmxAtlas, SpriteFrame, blit_sprite};

pub fn scale_den_for(cols: usize, rows: usize, t_left: usize) -> usize {
    let base = if rows <= 16 || cols < 44 { 50 } else { 40 };
    if (6..12).contains(&t_left) {
        300usize.div_ceil(t_left.saturating_sub(2).max(5)).max(base)
    } else {
        base
    }
}

pub fn x_geometry(
    cols: usize,
    rows: usize,
    (t_left, _, _, t_bot_row): (usize, usize, usize, usize),
) -> (usize, isize, isize, isize, isize, isize) {
    let buf_h = rows.saturating_mul(8).max(8);
    let d = scale_den_for(cols, rows, t_left);
    let (x_w, x_h, bo) = ((1200 / d).max(4), (1360 / d).max(4), (720 / d) as isize);
    let x_cols = x_w.div_ceil(4);
    let base_col = if t_left > x_cols + 2 {
        (t_left - x_cols - 2).clamp(1, 12)
    } else {
        1
    };
    let base_x = (base_col * 4) as isize;
    let floor_py = ((t_bot_row + 1) * 8).min(buf_h) as isize;
    let top_py = (floor_py - x_h as isize).max(0);
    let foot_py = (top_py + x_h as isize).min(buf_h as isize);
    let bx = base_x + (1080 / d) as isize;
    (d, base_x, foot_py, top_py, bx, top_py + bo)
}

pub fn destroy_front_for(
    cols: usize,
    rows: usize,
    text_bounds: (usize, usize, usize, usize),
    frame: u32,
) -> isize {
    if frame < 37 {
        -100
    } else if frame <= 64 {
        let (d, _, _, _, bx, _) = x_geometry(cols, rows, text_bounds);
        (bx + (((cols as isize * 4 + 64) - bx) * (frame - 37) as isize) / 24 + (960 / d) as isize)
            / 4
    } else {
        (cols as isize) + 100
    }
}

#[rustfmt::skip]
pub fn compose_frame_pixels(
    px_buf: &mut [u8],
    (cols, rows): (usize, usize),
    text_bounds: (usize, usize, usize, usize),
    frame: u32,
    a: &MmxAtlas,
) {
    let (buf_w, buf_h) = (cols * 4, rows * 8);
    px_buf.fill(0);
    let (d, base_x, foot_py, top_py, buster_x, buster_y) = x_geometry(cols, rows, text_bounds);
    let x_w = (1200 / d).max(4) as isize;
    let mut draw = |sp: &SpriteFrame, x: isize, y: isize, m: u8, den: usize| {
        blit_sprite(px_buf, buf_w, buf_h, sp, x, y, m, den);
    };

    if frame <= 8 {
        let bx = base_x + (x_w - (320 / d).max(4) as isize) / 2;
        let ry = ((foot_py * frame as isize) / 6).min(foot_py);
        draw(&a.beam, bx, ry - (1920 / d).max(16) as isize, 0, d);
        if frame >= 6 {
            let (sw, sh) = ((880 / d) as isize, (1160 / d) as isize);
            draw(&a.splash, base_x + (x_w - sw) / 2, foot_py - sh, 0, d);
        }
    } else if frame <= 13 {
        let sp = if frame <= 11 { &a.mat1 } else { &a.mat2 };
        let mh = ((sp.height * 40) / d) as isize;
        let m = u8::from(frame == 9 || frame == 12);
        draw(sp, base_x, foot_py - mh, m, d);
    } else if frame <= 16 {
        draw(&a.idle, base_x, top_py, 0, d);
    } else if frame <= 36 {
        let ct = frame - 17;
        let mode = if ct >= 9 {
            [2, 0, 1, 0][(ct % 4) as usize]
        } else {
            u8::from(ct.is_multiple_of(2))
        };
        draw(&a.aim, base_x, top_py, mode, d);
        let od = d * 2 + ((12 - (ct % 5) * 3) as usize * d) / 10;
        let orb_w = ((a.orb.width * 40) / od).max(4) as isize;
        let r = orb_w / 2;
        let max_sx = if text_bounds.0 >= 6 {
            (text_bounds.0 * 4) as isize
        } else {
            buf_w as isize
        };
        let ox = (buster_x - r).min(max_sx - orb_w - 1).max(0);
        let om = u8::from(ct >= 9 && ct.is_multiple_of(2)) * 3;
        draw(&a.orb, ox, buster_y - r, om, od);
        paint_charge_sparks(px_buf, (buf_w, buf_h), (ox + r, buster_y), max_sx, ct, d);
    } else if frame <= 64 {
        let ft = frame - 37;
        let (pose, rec, m) = if ft < 7 {
            (&a.fire, -4 * isize::from(ft < 4), u8::from(ft < 2))
        } else {
            (&a.aim, 0, 0)
        };
        draw(pose, (base_x + rec).max(0), top_py, m, d);
        let sx = buster_x + (((buf_w as isize + 64) - buster_x) * ft as isize) / 24;
        let sp = if ft.is_multiple_of(2) { &a.spiral2 } else { &a.spiral1 };
        let mid_y = ((text_bounds.2 * 8 + (text_bounds.3 + 1) * 8) / 2) as isize;
        let sd = d + ((3 - ft.min(3)) as usize * d) / 2;
        let sy = buster_y - ((sp.height * 20) / sd) as isize;
        paint_explosions(px_buf, (buf_w, buf_h), a, (sx, mid_y), text_bounds, ft, d);
        blit_sprite(px_buf, buf_w, buf_h, sp, sx, sy, u8::from(!ft.is_multiple_of(2)) * 3, sd);
    } else if frame <= 70 {
        draw(&a.idle, base_x, top_py, 0, d);
        paint_debris(px_buf, (buf_w, buf_h), text_bounds, frame - 37);
    } else if frame <= 76 {
        let sp = if frame <= 72 {
            &a.mat2
        } else if frame <= 74 {
            &a.mat1
        } else {
            &a.splash
        };
        let (mw, mh) = (((sp.width * 40) / d) as isize, ((sp.height * 40) / d) as isize);
        draw(sp, base_x + (x_w - mw) / 2, foot_py - mh, u8::from(frame % 2 == 1), d);
    } else if frame <= 84 {
        let bx = base_x + (x_w - (320 / d).max(4) as isize) / 2;
        let by = foot_py - (foot_py * (frame - 76) as isize) / 7;
        draw(&a.beam, bx, by - (1920 / d) as isize, 0, d);
    }
}

fn paint_charge_sparks(
    px_buf: &mut [u8],
    (buf_w, buf_h): (usize, usize),
    (cx, cy): (isize, isize),
    max_sx: isize,
    t: u32,
    scale_den: usize,
) {
    #[rustfmt::skip]
    const DIRS: [(isize, isize); 8] = [
        (10, 0), (-10, 0), (0, 10), (0, -10), (7, 7), (-7, 7), (7, -7), (-7, -7),
    ];
    let pal = if t >= 9 {
        [36, 32, 22, 35]
    } else {
        [22, 36, 20, 20]
    };
    let lim_x = max_sx.min(buf_w as isize);
    for ring in 0..2u32 {
        let phase = (t * 3 + ring * 5) % 10;
        let rad = (((10 - phase as isize).max(2)) * 32) / scale_den as isize;
        let col = pal[((ring + t) % 4) as usize];
        for &(dx, dy) in &DIRS {
            let (px, py) = (cx + (dx * rad) / 10, cy + (dy * rad) / 10);
            for (ox, oy) in [(0, 0), (1, 0), (0, 1)] {
                let (sx, sy) = (px + ox, py + oy);
                if sx >= 0 && sx < lim_x && sy >= 0 && sy < buf_h as isize {
                    px_buf[(sy as usize) * buf_w + (sx as usize)] = col;
                }
            }
        }
    }
}

#[inline]
#[rustfmt::skip]
fn stamp_spark(px_buf: &mut [u8], buf_w: usize, buf_h: usize, x: isize, y: isize, col: u8) {
    for (ox, oy, c) in [(0, 0, 22), (1, 0, col), (-1, 0, col), (0, 1, col), (0, -1, col)] {
        let (px, py) = (x + ox, y + oy);
        if px >= 0 && px < buf_w as isize && py >= 0 && py < buf_h as isize {
            px_buf[(py as usize) * buf_w + (px as usize)] = c;
        }
    }
}

fn paint_explosions(
    px: &mut [u8],
    wh: (usize, usize),
    a: &MmxAtlas,
    (shot_x, mid_y): (isize, isize),
    bounds: (usize, usize, usize, usize),
    fire_t: u32,
    d: usize,
) {
    let (t_left, t_right, t_top_row, t_bot_row) = bounds;
    let (sub_left, sub_right) = ((t_left * 4) as isize, ((t_right + 1) * 4) as isize);
    let front = shot_x + ((20 * 40) / d) as isize;
    let h = (((t_bot_row.saturating_sub(t_top_row) * 8) as isize) / 2).max(16);
    let offsets = [-(h * 4) / 5, (h * 4) / 5, -(h * 2) / 5, (h * 2) / 5, 0];
    let step_x = ((14 * 40) / d).max(12) as isize;
    for k in 1..6isize {
        let ex = front - k * step_x;
        if ex >= sub_left && ex <= sub_right + 12 {
            let wave_dy = offsets[((k + fire_t as isize) % 5) as usize];
            let sp = if (k + fire_t as isize) % 2 == 0 {
                &a.exp2
            } else {
                &a.exp1
            };
            let ey = mid_y + wave_dy - ((sp.height * 20) / d) as isize;
            blit_sprite(px, wh.0, wh.1, sp, ex, ey, 0, d);
        }
    }
    paint_debris(px, wh, bounds, fire_t);
}

#[rustfmt::skip]
fn paint_debris(
    px: &mut [u8],
    wh: (usize, usize),
    (t_left, t_right, t_top_row, t_bot_row): (usize, usize, usize, usize),
    fire_t: u32,
) {
    let (sub_left, sub_top) = ((t_left * 4) as isize, (t_top_row * 8) as isize);
    let span_x = (t_right.saturating_sub(t_left).max(1) * 4) as isize;
    let span_y = (t_bot_row.saturating_sub(t_top_row).max(2) * 8) as isize;
    for i in 0..18isize {
        let trig = 2 + ((i * 19) / 18) as u32;
        if fire_t >= trig && fire_t <= trig + 13 {
            let age = (fire_t - trig) as isize;
            let (ox, oy) = (sub_left + 8 + (i * span_x) / 18, sub_top + (i * 29) % span_y);
            let (vx, vy) = ((((i * 5) % 7) - 1) * 4, -((((i * 3) % 5) + 2) * 4));
            let (x, y) = (ox + vx * age, oy + vy * age + (age * age * 4) / 3);
            let col = [22, 26, 25, 23, 30][(age as usize / 3).min(4)];
            stamp_spark(px, wh.0, wh.1, x, y, col);
        }
    }
}
