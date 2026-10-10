use super::Gnats;
use super::render_effects::{draw_connectors, draw_stars};
use crate::runner::TerminalCell;

/// Soft glow sample for bright fireflies (orthogonal neighbors).
// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
fn paint_glow(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    px: i32,
    py: i32,
    color: (u8, u8, u8),
    intensity: f32,
    fade: f32,
) {
    let glow = intensity * 0.28 * fade;
    if glow < 0.05 {
        return;
    }
    let gr = (color.0 as f32 * glow) as u8;
    let gg = (color.1 as f32 * glow) as u8;
    let gb = (color.2 as f32 * glow) as u8;
    for (dx, dy, ch) in [(-1, 0, '·'), (1, 0, '·'), (0, -1, '·'), (0, 1, '·')] {
        let gx = px + dx;
        let gy = py + dy;
        if gx >= 0 && gx < cols as i32 && gy >= 0 && gy < rows as i32 {
            let idx = gy as usize * cols + gx as usize;
            if grid[idx].ch == ' ' || grid[idx].ch == '·' {
                grid[idx] = TerminalCell {
                    ch,
                    fg: (gr, gg, gb),
                    bg: (0, 0, 0),
                    bold: false,
                };
            }
        }
    }
}

pub fn draw_gnats(gnats: &Gnats, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    if cols == 0 || rows == 0 {
        return;
    }

    let accent = gnats.accent;
    let fade = gnats.intro_fade.clamp(0.0, 1.0);

    // 1. Clear grid (screen starts black)
    for cell in grid.iter_mut() {
        *cell = TerminalCell {
            ch: ' ',
            fg: (0, 0, 0),
            bg: (0, 0, 0),
            bold: false,
        };
    }

    // 1b. Draw distant backdrop stars with lens flares when excited by fireflies
    draw_stars(
        &gnats.stars,
        gnats.time_elapsed,
        cols,
        rows,
        accent,
        fade,
        grid,
    );

    // 2. Draw wireframe network connector lines (depth-faded)
    draw_connectors(&gnats.fireflies, cols, rows, fade, grid);

    // 3. Draw firefly history trails
    for p in &gnats.fireflies {
        let h_len = p.history.len();
        for (k, &(hx, hy)) in p.history.iter().enumerate() {
            if hx >= 0 && hx < cols as i32 && hy >= 0 && hy < rows as i32 {
                let idx = hy as usize * cols + hx as usize;
                if grid[idx].ch == ' ' {
                    // Trail fades as it goes further back in time
                    let t = (k + 1) as f32 / (h_len + 1) as f32;
                    let intensity = t * 0.35 * fade;
                    let tr = (p.color.0 as f32 * intensity) as u8;
                    let tg = (p.color.1 as f32 * intensity) as u8;
                    let tb = (p.color.2 as f32 * intensity) as u8;

                    grid[idx] = TerminalCell {
                        ch: '·',
                        fg: (tr, tg, tb),
                        bg: (0, 0, 0),
                        bold: false,
                    };
                }
            }
        }
    }

    // 3.5. Draw kill sparks
    for spark in &gnats.kill_sparks {
        let sx = spark.x.round() as i32;
        let sy = spark.y.round() as i32;
        if sx >= 0 && sx < cols as i32 && sy >= 0 && sy < rows as i32 {
            let idx = sy as usize * cols + sx as usize;
            if grid[idx].ch == ' '
                || grid[idx].ch == '·'
                || grid[idx].ch == '─'
                || grid[idx].ch == '│'
                || grid[idx].ch == '╱'
                || grid[idx].ch == '╲'
            {
                let life = (spark.life.min(1.0) * fade).clamp(0.0, 1.0);
                grid[idx] = TerminalCell {
                    ch: '*',
                    fg: (
                        (spark.color.0 as f32 * life) as u8,
                        (spark.color.1 as f32 * life) as u8,
                        (spark.color.2 as f32 * life) as u8,
                    ),
                    bg: (0, 0, 0),
                    bold: spark.life > 0.4,
                };
            }
        }
    }

    // 4. Soft glow under bright (on-phase) fireflies, then bodies
    for p in &gnats.fireflies {
        let blink = f32::midpoint(
            (gnats.time_elapsed * p.blink_rate + p.blink_phase).sin(),
            1.0,
        );
        // Desynced blink: most of the time dim, brief bright peaks
        let on = blink > 0.62;
        let brightness = if on {
            0.55 + 0.45 * ((blink - 0.62) / 0.38).clamp(0.0, 1.0)
        } else {
            0.12 + 0.22 * blink
        };

        let px = p.x.round() as i32;
        let py = p.y.round() as i32;
        if px >= 0 && px < cols as i32 && py >= 0 && py < rows as i32 {
            if on && brightness > 0.7 {
                paint_glow(grid, cols, rows, px, py, p.color, brightness, fade);
            }
            let idx = py as usize * cols + px as usize;
            let ch = match p.size {
                3 => '✦',
                2 => 'o',
                1 => '+',
                _ => '·',
            };
            let fr = (p.color.0 as f32 * brightness * fade) as u8;
            let fg = (p.color.1 as f32 * brightness * fade) as u8;
            let fb = (p.color.2 as f32 * brightness * fade) as u8;
            grid[idx] = TerminalCell {
                ch,
                fg: (fr, fg, fb),
                bg: (0, 0, 0),
                bold: on && brightness > 0.65,
            };
        }
    }

    // 5. Draw Attractors as faint pulsing halo flares
    for (i, attr) in gnats.attractors.iter().enumerate() {
        let ax = attr.x.round() as i32;
        let ay = attr.y.round() as i32;
        if ax >= 0 && ax < cols as i32 && ay >= 0 && ay < rows as i32 {
            let idx = ay as usize * cols + ax as usize;

            // Pulsing indicator char
            let pulse = (gnats.time_elapsed * 3.0 + i as f32 * 1.5).sin();
            let ch = if pulse > 0.5 {
                '¤'
            } else if pulse > -0.5 {
                '☼'
            } else {
                'o'
            };

            // Soft color intensity scaled by intro fade
            let att_r = (attr.color.0 as f32 * 0.4 * fade) as u8;
            let att_g = (attr.color.1 as f32 * 0.4 * fade) as u8;
            let att_b = (attr.color.2 as f32 * 0.4 * fade) as u8;

            grid[idx] = TerminalCell {
                ch,
                fg: (att_r, att_g, att_b),
                bg: (0, 0, 0),
                bold: false,
            };
        }
    }

    // 6. Draw centered logo with glow excitation
    // library 4.1: render the system logo from the live OS info
    // (replaces pre-4.1 `trance_core::logo_lines()` + `logo_dimensions()`).
    if let Some(logo) = crate::runner::place_centered_logo(cols, rows, &gnats.logo_text, None) {
        let logo_w = logo.width;
        for (r_offset, line) in logo.lines.iter().enumerate() {
            let gy = logo.y + r_offset;
            if gy >= rows {
                continue;
            }
            for (c_offset, ch) in line.chars().enumerate() {
                let gx = logo.x + c_offset;
                if gx >= cols {
                    continue;
                }
                if ch != ' ' {
                    let l_idx = r_offset * logo_w + c_offset;
                    let exc = gnats.logo_excitation.get(l_idx).copied().unwrap_or(0.0);

                    let (fg, bold) = if exc > 0.05 {
                        let t = exc;
                        let r = ((accent.0 as f32 * t + 255.0 * (1.0 - t)) * fade).min(255.0) as u8;
                        let g = ((accent.1 as f32 * t + 255.0 * (1.0 - t)) * fade).min(255.0) as u8;
                        let b = ((accent.2 as f32 * t + 255.0 * (1.0 - t)) * fade).min(255.0) as u8;
                        ((r, g, b), true)
                    } else {
                        (
                            (
                                (accent.0 as f32 * 0.25 * fade) as u8,
                                (accent.1 as f32 * 0.25 * fade) as u8,
                                (accent.2 as f32 * 0.25 * fade) as u8,
                            ),
                            false,
                        )
                    };

                    grid[gy * cols + gx] = TerminalCell {
                        ch,
                        fg,
                        bg: (0, 0, 0),
                        bold,
                    };
                }
            }
        }
    }
}
