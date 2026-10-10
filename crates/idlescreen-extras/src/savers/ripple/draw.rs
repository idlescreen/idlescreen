//! Terminal cell drawing logic for Ripple screensaver.

use super::physics::{stroke_ellipse, wave_displace};
use super::types::{Drop, Ring, Splash};
use crate::runner::TerminalCell;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_impl(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    rings: &[Ring],
    drops: &[Drop],
    splashes: &[Splash],
    intro_fade: f32,
    surface_phase: f32,
    weather_intensity: f32,
    wind: f32,
    accent: (u8, u8, u8),
    logo_text: &str,
    accum: &std::cell::RefCell<Vec<f32>>,
    accent_a: &std::cell::RefCell<Vec<f32>>,
) {
    if cols == 0 || rows == 0 || grid.is_empty() {
        return;
    }
    let fade = intro_fade.clamp(0.0, 1.0);
    let secondary = crate::runner::is_secondary_monitor();
    let (ar, ag, ab) = accent;
    let n = cols * rows;

    // Reuse the caller-owned scratch buffers — this ran two full-grid
    // allocations per frame (~80KB @ 1080p-equivalent, ~5MB/s churn).
    let mut accum = accum.borrow_mut();
    let mut accent_a = accent_a.borrow_mut();
    accum.clear();
    accum.resize(n, 0.0);
    accent_a.clear();
    accent_a.resize(n, 0.0);

    let aspect = 0.5f32;
    for ring in rings {
        let a = ring.life * ring.strength * fade;
        if a < 0.02 {
            continue;
        }
        let rx = ring.r;
        let ry = ring.r * aspect;
        let accent_w = if ring.age < 0.35 {
            (1.0 - ring.age / 0.35) * 1.2
        } else {
            0.0
        };
        stroke_ellipse(
            &mut accum,
            &mut accent_a,
            cols,
            rows,
            ring.x,
            ring.y,
            rx,
            ry,
            a,
            accent_w,
            wind,
        );
        if ring.r > 1.5 {
            stroke_ellipse(
                &mut accum,
                &mut accent_a,
                cols,
                rows,
                ring.x,
                ring.y,
                rx + 0.55,
                ry + 0.3,
                a * 0.35,
                accent_w * 0.4,
                wind,
            );
        }
    }

    for y in 0..rows {
        for x in 0..cols {
            let cell = &mut grid[y * cols + x];
            cell.bold = false;
            let i = y * cols + x;
            if secondary {
                cell.ch = ' ';
                cell.fg = (0, 0, 0);
                cell.bg = (0, 0, 0);
                continue;
            }
            let depth = y as f32 / (rows as f32).max(1.0);
            let shimmer = 0.5 + 0.5 * ((x as f32 * 0.15 + surface_phase + y as f32 * 0.08).sin());
            let sky = 1.0 - weather_intensity * 0.12;
            let base_r = (3.0 + depth * 4.0 + shimmer * 2.0) * sky;
            let base_g = (10.0 + depth * 18.0 + shimmer * 6.0) * sky;
            let base_b = (22.0 + depth * 40.0 + shimmer * 10.0) * sky;
            let mix = 0.08;
            let mut bg = (
                ((base_r * (1.0 - mix) + ar as f32 * mix) * fade).min(255.0) as u8,
                ((base_g * (1.0 - mix) + ag as f32 * mix) * fade).min(255.0) as u8,
                ((base_b * (1.0 - mix) + ab as f32 * mix) * fade).min(255.0) as u8,
            );

            let energy = accum[i];
            let acc = accent_a[i];
            if energy > 0.04 {
                let crest = (energy / 1.15).clamp(0.0, 1.6);
                let a = crest.min(1.0);
                let accent_mix = (acc / energy.max(0.01)).clamp(0.0, 1.0);
                let bright = 0.4 + 0.6 * a;
                let boost = if energy > 1.05 { 1.25 } else { 1.0 };
                let fr = (90.0 * bright * (1.0 - accent_mix)
                    + ar as f32 * (0.35 + 0.65 * accent_mix))
                    * a
                    * boost;
                let fg_g = (160.0 * bright * (1.0 - accent_mix)
                    + ag as f32 * (0.35 + 0.65 * accent_mix))
                    * a
                    * boost;
                let fb = (220.0 * bright * (1.0 - accent_mix * 0.5)
                    + ab as f32 * (0.25 + 0.55 * accent_mix))
                    * a
                    * boost;
                cell.ch = if energy > 1.2 {
                    'o'
                } else if energy > 0.7 {
                    '·'
                } else if energy > 0.25 {
                    '.'
                } else {
                    '·'
                };
                cell.fg = (
                    (fr * fade).min(255.0) as u8,
                    (fg_g * fade).min(255.0) as u8,
                    (fb * fade).min(255.0) as u8,
                );
                cell.bold = energy > 1.1 || accent_mix > 0.5;
                bg = (
                    ((bg.0 as f32 + fr * 0.08).min(255.0)) as u8,
                    ((bg.1 as f32 + fg_g * 0.08).min(255.0)) as u8,
                    ((bg.2 as f32 + fb * 0.1).min(255.0)) as u8,
                );
            } else {
                cell.ch = ' ';
                cell.fg = bg;
            }
            cell.bg = bg;
        }
    }
    if secondary {
        return;
    }

    for d in drops {
        let x = d.x as usize;
        let y = d.y as usize;
        if x >= cols || y >= rows {
            continue;
        }
        let cell = &mut grid[y * cols + x];
        cell.ch = '|';
        cell.fg = (
            ((140.0 + ar as f32 * 0.2) * fade).min(255.0) as u8,
            ((190.0 + ag as f32 * 0.15) * fade).min(255.0) as u8,
            ((230.0) * fade) as u8,
        );
        if y > 0 {
            let t = &mut grid[(y - 1) * cols + x];
            if t.ch == ' ' || t.ch == '·' || t.ch == '.' {
                t.ch = '\'';
                t.fg = (
                    (80.0 * fade) as u8,
                    (120.0 * fade) as u8,
                    (160.0 * fade) as u8,
                );
            }
        }
    }

    for s in splashes {
        let x = s.x as usize;
        let y = s.y as usize;
        if x >= cols || y >= rows {
            continue;
        }
        let a = (s.life / 0.3).clamp(0.0, 1.0) * fade;
        let cell = &mut grid[y * cols + x];
        cell.ch = '*';
        cell.fg = (
            ((ar as f32 * 0.6 + 120.0) * a).min(255.0) as u8,
            ((ag as f32 * 0.6 + 140.0) * a).min(255.0) as u8,
            ((ab as f32 * 0.5 + 180.0) * a).min(255.0) as u8,
        );
        cell.bold = true;
    }

    if let Some(logo) = crate::runner::place_centered_logo(cols, rows, logo_text, None) {
        for (r_offset, line) in logo.lines.iter().enumerate() {
            let base_y = logo.y + r_offset;
            for (c_offset, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let base_x = logo.x + c_offset;
                if base_x >= cols || base_y >= rows {
                    continue;
                }
                let (wdx, wdy, hit) =
                    wave_displace(base_x as f32 + 0.5, base_y as f32 + 0.5, rings);
                let nx = (base_x as f32 + wdx).round() as isize;
                let ny = (base_y as f32 + wdy).round() as isize;
                if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                    continue;
                }
                let cell = &mut grid[ny as usize * cols + nx as usize];
                if cell.ch == '|' || cell.ch == '*' {
                    continue;
                }
                let base = 0.16 + hit * 0.55;
                let a = (base * fade).clamp(0.0, 1.0);
                let accent_m = (hit * 0.85).clamp(0.0, 1.0);
                cell.ch = ch;
                cell.fg = (
                    ((28.0 + ar as f32 * (0.25 + 0.75 * accent_m)) * a * 1.4).min(255.0) as u8,
                    ((36.0 + ag as f32 * (0.25 + 0.75 * accent_m)) * a * 1.4).min(255.0) as u8,
                    ((48.0 + ab as f32 * (0.25 + 0.75 * accent_m)) * a * 1.4).min(255.0) as u8,
                );
                cell.bold = hit > 0.45;
            }
        }
    }
}
