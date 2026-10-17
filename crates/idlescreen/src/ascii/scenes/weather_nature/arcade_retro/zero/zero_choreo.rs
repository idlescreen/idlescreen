// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 5-act SNES Mega Man X3 Zero multi-frame choreography: Crimson Teleport In,
//! Flowing Ponytail Idle, Twin Z-Buster Combo, Z-Saber 3-Stage Slash + Animated
//! Crescent Wave + Maverick Explosions, and Teleport Out.

use super::zero_data::{SpriteFrame, ZeroAtlas, blit_sprite};

type Bounds = (usize, usize, usize, usize);
type Dim = (usize, usize);
type Pt = (isize, isize);

pub fn scale_den_for(cols: usize, rows: usize, t_left: usize) -> usize {
    let base = if rows <= 16 || cols < 44 { 50 } else { 40 };
    if (6..13).contains(&t_left) {
        380usize.div_ceil(t_left.saturating_sub(2).max(5)).max(base)
    } else {
        base
    }
}

pub fn zero_geometry((cols, rows): Dim, (t_left, _, _, t_bot_row): Bounds) -> [isize; 6] {
    let buf_h = rows.saturating_mul(8).max(8);
    let d = scale_den_for(cols, rows, t_left);
    let (z_w, z_h, bo) = ((1520 / d).max(4), (1760 / d).max(4), (840 / d) as isize);
    let z_cols = z_w.div_ceil(4);
    let base_col = if t_left > z_cols + 1 {
        (t_left - z_cols - 1).clamp(3, 12)
    } else {
        1
    };
    let base_x = (base_col * 4) as isize;
    let floor_py = ((t_bot_row + 1) * 8).min(buf_h) as isize;
    let top_py = (floor_py - z_h as isize).max(0);
    let foot_py = (top_py + z_h as isize).min(buf_h as isize);
    [
        d as isize,
        base_x,
        foot_py,
        top_py,
        base_x + (1640 / d) as isize,
        top_py + bo,
    ]
}

pub fn destroy_front_for_row(cr: Dim, bounds: Bounds, y_row: usize, frame: u32) -> isize {
    let (t_left, t_right, _, t_bot_row) = bounds;
    let slant = (t_bot_row.saturating_sub(y_row) as isize) / 3;
    let buster_max = (t_left as isize) + 14 - slant;
    if frame < 30 {
        -100
    } else if frame <= 42 {
        (t_left as isize) + ((frame - 30) as isize) / 2 - slant
    } else if frame <= 66 {
        (t_left as isize) + 6 + ((frame - 43).min(12) as isize * 2) / 3 - slant
    } else if frame <= 100 {
        let base = (t_left as isize) + 10;
        let span = (t_right as isize + 18) - base;
        (base + (span * (frame - 66) as isize) / 31 + slant).max(buster_max)
    } else {
        (cr.0 as isize) + 100
    }
}

#[rustfmt::skip]
pub fn compose_frame_pixels(px_buf: &mut [u8], cr: Dim, bounds: Bounds, frame: u32, a: &ZeroAtlas) {
    let wh @ (buf_w, buf_h) = (cr.0 * 4, cr.1 * 8);
    px_buf.fill(0);
    let [d_i, base_x, foot_py, _, buster_x, buster_y] = zero_geometry(cr, bounds);
    let d = d_i as usize;
    let z_w = (1520 / d).max(4) as isize;
    let mid_y = ((bounds.2 * 8 + (bounds.3 + 1) * 8) / 2) as isize;
    let slash_y = (foot_py - ((22 * 40) / d) as isize).max(0);
    let gy = |sp: &SpriteFrame| (foot_py - ((sp.height * 40) / d).max(1) as isize).max(0);
    let ox = |dx: isize| base_x + (dx * 40) / d_i;
    let idle = |t: u32| if t.is_multiple_of(2) { (&a.idle1, base_x) } else { (&a.idle2, ox(-3)) };
    let mut draw = |sp: &SpriteFrame, x: isize, y: isize, m: u8| {
        blit_sprite(px_buf, buf_w, buf_h, sp, x, y, m, d);
    };

    if frame <= 9 {
        let (bw, bh) = (((a.beam.width * 40) / d).max(4) as isize, ((a.beam.height * 40) / d).max(16) as isize);
        let ry = ((foot_py * frame as isize) / 7).min(foot_py);
        draw(&a.beam, base_x + (z_w - bw) / 2, ry - bh, 0);
        if frame >= 6 {
            let sw = ((a.splash.width * 40) / d) as isize;
            draw(&a.splash, base_x + (z_w - sw) / 2, gy(&a.splash), 0);
        }
    } else if frame <= 18 {
        let (sp, x) = match frame { 10..=12 => (&a.mat1, ox(-9)), 13..=15 => (&a.mat2, ox(-9)), _ => (&a.mat3, ox(-9)) };
        draw(sp, x, gy(sp), u8::from(frame == 10 || frame == 13));
    } else if frame <= 26 {
        let (sp, ix) = idle(frame / 2);
        draw(sp, ix, gy(sp), 0);
    } else if frame <= 42 {
        let bt = (frame - 27) as isize;
        let (pose, px_x, m) = match bt {
            0 | 1 => (&a.buster1, ox(-5), 1), 2..=9 => (&a.buster1, ox(-4), 0),
            10..=12 => (&a.idle2, ox(-3), u8::from(bt == 12)),
            _ => (&a.buster1, ox(-4), u8::from(bt == 14)),
        };
        draw(pose, px_x, gy(pose), m);
        let tx = (bounds.0 * 4) as isize + (buf_w as isize) / 5;
        let sx = buster_x + ((tx - buster_x) * bt.min(11)) / 11;
        let sy = buster_y - ((a.shot1.height * 20) / d) as isize;
        if bt <= 11 { draw(&a.shot1, sx, sy, 0); }
        if bt >= 8 { draw(&a.slash_exp2, sx + 8, buster_y - ((a.slash_exp2.height * 20) / d) as isize, u8::from(bt % 2 == 1) * 3); }
        paint_sparks(px_buf, wh, (sx + 18, buster_y), bt as u32, false);
    } else if frame <= 58 {
        let bt = (frame - 43) as isize;
        let (pose, px_x, m) = match bt {
            0..=2 => (&a.buster2, ox(-15), u8::from(bt < 2)), 3..=10 => (&a.buster2, ox(-14), 0),
            11..=13 => (&a.buster1, ox(-4), 0), _ => (&a.idle2, ox(-3), 2),
        };
        draw(pose, px_x, gy(pose), m);
        let tx = (bounds.0 * 4) as isize + (buf_w as isize * 2) / 5;
        let sx = buster_x + ((tx - buster_x) * bt.min(11)) / 10;
        let sy = buster_y - ((a.shot2.height * 20) / d) as isize;
        if bt <= 11 { draw(&a.shot2, sx, sy, 0); }
        if bt >= 7 {
            let ey = buster_y - ((a.slash_exp1.height * 20) / d) as isize;
            draw(&a.slash_exp2, sx + 10, ey + 2, 0);
            draw(&a.slash_exp1, sx - 4, ey - 2, 3);
        }
        paint_sparks(px_buf, wh, (sx + 22, buster_y), bt as u32, true);
    } else if frame <= 66 {
        let pt = frame - 59;
        let (sp, x) = if pt < 2 { (&a.idle1, base_x) } else { (&a.saber_prep, ox(-16)) };
        draw(sp, x, gy(sp), u8::from(pt.is_multiple_of(2)) * 2);
    } else if frame <= 100 {
        let st = (frame - 67) as isize;
        let (pose, px_x, m) = match st {
            0..=3 => (&a.saber_swing1, ox(-12), u8::from(st < 2) * 2),
            4..=9 => (&a.saber_swing2, ox(-18), u8::from(st == 4) * 2),
            10..=21 => (&a.saber_pose, ox(-12), 0),
            _ => { let (sp, ix) = idle((st / 3) as u32); (sp, ix, 0) }
        };
        draw(pose, px_x, gy(pose), m);
        if st >= 4 {
            let wt = st - 4;
            let wave_sp = if (wt / 2) % 2 == 0 { &a.wave1 } else { &a.wave2 };
            let wx = buster_x + 12 + (((buf_w as isize + 56) - buster_x) * wt) / 27;
            let wy = slash_y - ((wave_sp.height * 20) / d) as isize;
            paint_pillars(px_buf, wh, a, (wx, mid_y), bounds, wt as u32, d);
            blit_sprite(px_buf, buf_w, buf_h, wave_sp, wx, wy, 0, d);
        }
    } else if frame <= 107 {
        let (sp, ix) = idle(frame / 2);
        draw(sp, ix, gy(sp), 0);
        paint_shards(px_buf, wh, bounds, frame - 69);
    } else if frame <= 114 {
        let (sp, x) = match frame {
            108 | 109 => (&a.mat3, ox(-9)),
            110 | 111 => (&a.mat2, ox(-9)),
            112 | 113 => (&a.mat1, ox(-9)),
            _ => (&a.splash, base_x + (z_w - ((a.splash.width * 40) / d) as isize) / 2),
        };
        draw(sp, x, gy(sp), u8::from(frame % 2 == 1));
    } else if frame <= 120 {
        let (bw, bh) = (((a.beam.width * 40) / d).max(4) as isize, ((a.beam.height * 40) / d).max(16) as isize);
        let by = foot_py - (foot_py * (frame - 114) as isize) / 6;
        draw(&a.beam, base_x + (z_w - bw) / 2, by - bh, 0);
    }
}

#[inline]
#[rustfmt::skip]
fn stamp(px_buf: &mut [u8], (w, h): Dim, x: isize, y: isize, sz: isize, col: u8) {
    let pts: &[(isize, isize, u8)] = if sz >= 2 {
        &[(0, 0, 22), (1, 0, col), (-1, 0, col), (0, 1, col), (0, -1, col)]
    } else {
        &[(0, 0, col)]
    };
    for &(ox, oy, c) in pts {
        let (px, py) = (x + ox, y + oy);
        if px >= 0 && px < w as isize && py >= 0 && py < h as isize {
            px_buf[(py as usize) * w + (px as usize)] = c;
        }
    }
}

fn paint_sparks(px: &mut [u8], wh: Dim, (cx, cy): Pt, t: u32, heavy: bool) {
    #[rustfmt::skip]
    const VECS: [Pt; 8] = [(3, -3), (4, 2), (-3, -4), (-4, 3), (2, -5), (5, 0), (-2, 4), (3, 4)];
    let cols = if heavy {
        [22, 30, 25, 26]
    } else {
        [19, 25, 24, 22]
    };
    let sz = if heavy { 2 } else { 1 };
    for (i, &(vx, vy)) in VECS.iter().enumerate() {
        let step = (t as isize + (i as isize % 3)).max(1);
        stamp(px, wh, cx + vx * step, cy + vy * step, sz, cols[i % 4]);
    }
}

fn paint_pillars(
    px: &mut [u8],
    wh: Dim,
    a: &ZeroAtlas,
    (wave_x, mid_y): Pt,
    bounds: Bounds,
    slash_t: u32,
    d: usize,
) {
    let (t_left, t_right, t_top_row, t_bot_row) = bounds;
    let (sub_left, sub_right) = ((t_left * 4) as isize, ((t_right + 1) * 4) as isize);
    let h = (((t_bot_row.saturating_sub(t_top_row) * 8) as isize) / 2).max(16);
    let offsets = [-(h * 3) / 5, (h * 3) / 5, -h / 4, h / 4, 0];
    let step_x = ((15 * 40) / d).max(14) as isize;
    for k in 1..6isize {
        let x = wave_x - k * step_x;
        if x >= sub_left && x <= sub_right + 12 {
            let dy = offsets[((k + slash_t as isize) % 5) as usize];
            let sp = if (k + slash_t as isize) % 2 == 0 {
                &a.slash_exp1
            } else {
                &a.slash_exp2
            };
            let y = mid_y + dy - ((sp.height * 20) / d) as isize;
            let mode = u8::from((k + slash_t as isize) % 3 == 0) * 3;
            blit_sprite(px, wh.0, wh.1, sp, x, y, mode, d);
        }
    }
    paint_shards(px, wh, bounds, slash_t);
}

fn paint_shards(px: &mut [u8], wh: Dim, (t_left, t_right, t_top, t_bot): Bounds, slash_t: u32) {
    let (sub_left, sub_top) = ((t_left * 4) as isize, (t_top * 8) as isize);
    let span_x = (t_right.saturating_sub(t_left).max(1) * 4) as isize;
    let span_y = (t_bot.saturating_sub(t_top).max(2) * 8) as isize;
    for i in 0..20isize {
        let trig = ((i * 24) / 20) as u32;
        if slash_t >= trig && slash_t <= trig + 16 {
            let age = (slash_t - trig) as isize;
            let (ox, oy) = (sub_left + (i * span_x) / 20, sub_top + (i * 31) % span_y);
            let dir = if i % 2 == 0 { -1 } else { 1 };
            let (vx, vy) = (((i * 5) % 7 - 1) * 4, dir * (((i * 3) % 5 + 2) * 4));
            let col = [33, 19, 29, 32, 28, 31][(age as usize + i as usize) % 6];
            stamp(px, wh, ox + vx * age, oy + vy * age, 3, col);
        }
    }
}
