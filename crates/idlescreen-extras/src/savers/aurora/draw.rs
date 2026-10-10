//! Aurora renderer: column-sweep field evaluation then row-major raster.

use super::Aurora;
use crate::runner::TerminalCell;

/// Intensity ramp — sparse enough that dim cells read as near-black.
const RAMP: &[char] = &[' ', '·', ':', '░', '▒', '▓', '█'];
/// Above-edge bleed multiplier — keeps a faint glow over the bright edge.
const LEAK: f32 = 0.35;
/// Aurora signature green; blended toward the theme accent per curtain.
const GREEN: (f32, f32, f32) = (60.0, 235.0, 130.0);
const SKY_BG: (u8, u8, u8) = (2, 4, 8);

pub fn draw_aurora(eff: &Aurora, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    if cols == 0 || rows == 0 || grid.len() < cols * rows {
        return;
    }
    let fade = eff.intro_fade.clamp(0.0, 1.0);
    let secondary = crate::runner::is_secondary_monitor();

    if secondary {
        // Dark sky + dim stars only on secondary displays.
        for cell in grid.iter_mut().take(cols * rows) {
            *cell = TerminalCell {
                ch: ' ',
                fg: (40, 44, 52),
                bg: SKY_BG,
                bold: false,
            };
        }
        draw_stars(eff, grid, cols, rows, 0.45);
        return;
    }

    // Pass 1 — per-column edge heights + ray modulation into scratch.
    let mut edges = eff.edge_scratch.borrow_mut();
    let mut rays = eff.ray_scratch.borrow_mut();
    let mut field = eff.field_scratch.borrow_mut();
    let mut hue = eff.hue_scratch.borrow_mut();
    field.clear();
    field.resize(cols * rows, 0.0);
    hue.clear();
    hue.resize(cols * rows, 0.0);
    super::sky::eval_columns(
        &eff.curtains,
        eff.time_elapsed,
        cols,
        rows,
        eff.quality_scale,
        &mut edges,
        &mut rays,
    );

    // Pass 2 — column sweep. Each curtain's intensity is a running
    // product: one exp() per column, then a multiply per row.
    let ncurt = eff.curtains.len();
    for x in 0..cols {
        for (i, c) in eff.curtains.iter().enumerate().take(ncurt) {
            let ray = rays[i * cols + x] * c.strength;
            if ray <= 0.001 {
                continue;
            }
            let e = edges[i * cols + x];
            let sigma_d = c.sigma.max(0.5);
            let sigma_u = (c.sigma * 0.45).max(0.5);
            let step_d = (-1.0 / sigma_d).exp();
            let step_u = (1.0 / sigma_u).exp();
            let mut iv = if e >= 0.0 {
                LEAK * (-e / sigma_u).exp()
            } else {
                (e / sigma_d).exp()
            };
            let mut below = e < 0.0;
            for y in 0..rows {
                let yf = y as f32;
                if !below && yf > e {
                    // Re-anchor once at the crossing row (iv ~= 1).
                    below = true;
                    iv = (-(yf - e) / sigma_d).exp();
                }
                let w = iv * ray;
                let idx = y * cols + x;
                field[idx] += w;
                hue[idx] += w * c.hue_mix;
                iv *= if below { step_d } else { step_u };
            }
        }
    }

    // Pass 3 — raster: intensity → ramp char, hue → green↔accent lerp.
    let boost = 1.0 + 0.55 * eff.surge_env;
    let (ar, ag, ab) = eff.accent;
    let (ar, ag, ab) = (ar as f32, ag as f32, ab as f32);
    for (idx, cell) in grid.iter_mut().enumerate().take(cols * rows) {
        let total = (field[idx] * boost * fade).clamp(0.0, 1.6);
        if total > 0.03 {
            let mix = (hue[idx] / field[idx].max(1e-6)).clamp(0.0, 1.0);
            let cr = GREEN.0 + (ar - GREEN.0) * mix;
            let cg = GREEN.1 + (ag - GREEN.1) * mix;
            let cb = GREEN.2 + (ab - GREEN.2) * mix;
            let lit = (0.35 + 0.65 * total.min(1.0)).min(1.0);
            let ramp_idx = ((total * (RAMP.len() - 1) as f32) as usize).min(RAMP.len() - 1);
            *cell = TerminalCell {
                ch: RAMP[ramp_idx],
                fg: (
                    (cr * lit).min(255.0) as u8,
                    (cg * lit).min(255.0) as u8,
                    (cb * lit).min(255.0) as u8,
                ),
                bg: (
                    (SKY_BG.0 as f32 + cr * total * 0.18).min(255.0) as u8,
                    (SKY_BG.1 as f32 + cg * total * 0.18).min(255.0) as u8,
                    (SKY_BG.2 as f32 + cb * total * 0.18).min(255.0) as u8,
                ),
                bold: total > 0.85,
            };
        } else {
            *cell = TerminalCell {
                ch: ' ',
                fg: (200, 205, 215),
                bg: SKY_BG,
                bold: false,
            };
        }
    }
    drop(edges);
    drop(rays);
    drop(field);
    drop(hue);

    draw_stars(eff, grid, cols, rows, 1.0);
    draw_logo(eff, grid, cols, rows);
}

/// Twinkling stars — only into cells the aurora left dark.
fn draw_stars(eff: &Aurora, grid: &mut [TerminalCell], cols: usize, rows: usize, dim: f32) {
    for star in &eff.stars {
        let sx = (star.x * cols as f32) as usize;
        let sy = (star.y * rows as f32) as usize;
        if sx >= cols || sy >= rows {
            continue;
        }
        let tw = 0.5 + 0.5 * (eff.time_elapsed * star.rate + star.phase).sin();
        if tw < 0.55 {
            continue;
        }
        let idx = sy * cols + sx;
        if grid[idx].ch != ' ' {
            continue; // aurora owns this cell
        }
        let l = ((tw - 0.55) / 0.45) * dim;
        grid[idx] = TerminalCell {
            ch: star.ch,
            fg: (
                (170.0 * l + 30.0) as u8,
                (185.0 * l + 32.0) as u8,
                (215.0 * l + 40.0) as u8,
            ),
            bg: SKY_BG,
            bold: false,
        };
    }
}

/// Centered OS caption; aurora glow shows through the cells' background.
fn draw_logo(eff: &Aurora, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    let Some(logo) = crate::runner::place_centered_logo(cols, rows, &eff.logo_text, None) else {
        return;
    };
    let (ar, ag, ab) = eff.accent;
    let fr = (ar as f32 * 0.55 + 115.0).min(255.0) as u8;
    let fg = (ag as f32 * 0.55 + 115.0).min(255.0) as u8;
    let fb = (ab as f32 * 0.55 + 115.0).min(255.0) as u8;
    for (dy, line) in logo.lines.iter().enumerate() {
        for (dx, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = logo.x + dx;
            let y = logo.y + dy;
            if x >= cols || y >= rows {
                continue;
            }
            let cell = &mut grid[y * cols + x];
            cell.ch = ch;
            cell.fg = (fr, fg, fb);
            cell.bold = true;
        }
    }
}
