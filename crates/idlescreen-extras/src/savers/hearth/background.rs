//! Hearth background, mantel, and hearthstone rendering helpers.

use crate::runner::TerminalCell;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_background(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    mantel_y: usize,
    fade: f32,
    fire_pulse: f32,
    coal_pulse: f32,
    coal_boost: f32,
    time: f32,
    accent: (u8, u8, u8),
) -> Option<usize> {
    let secondary = crate::runner::is_secondary_monitor();
    let (ar, ag, ab) = accent;
    let inv_glow_radius = 1.0 / (fire_w * 2.6).max(1e-6);

    for y in 0..rows {
        for x in 0..cols {
            let cell = &mut grid[y * cols + x];
            cell.bold = false;
            cell.ch = ' ';
            if secondary {
                cell.fg = (0, 0, 0);
                cell.bg = (0, 0, 0);
                continue;
            }
            let dx = x as f32 - fire_cx;
            let dy = y as f32 - fire_y;
            let dist = (dx * dx + dy * dy * 1.4).sqrt();
            // g^1.5 == g * sqrt(g); avoids powf in this per-cell hot loop.
            let g = (1.0 - dist * inv_glow_radius).clamp(0.0, 1.0);
            let glow = g * g.sqrt();
            let floor = if y as f32 > fire_y { 0.08 } else { 0.0 };
            let base_r = 6.0 + glow * 55.0 * fire_pulse + floor * 20.0;
            let base_g = 4.0 + glow * 22.0 * fire_pulse + floor * 8.0;
            let base_b = 8.0 + glow * 6.0;
            let mix = glow * 0.18;
            cell.bg = (
                ((base_r * (1.0 - mix) + ar as f32 * mix * 0.5) * fade).min(255.0) as u8,
                ((base_g * (1.0 - mix) + ag as f32 * mix * 0.5) * fade).min(255.0) as u8,
                ((base_b * (1.0 - mix) + ab as f32 * mix) * fade).min(255.0) as u8,
            );
            cell.fg = cell.bg;
        }
    }
    if secondary {
        return None;
    }

    let arch_top = mantel_y;
    let arch_bot = (fire_y as usize).min(rows.saturating_sub(1));
    let left = (fire_cx - fire_w * 0.58).round() as isize;
    let right = (fire_cx + fire_w * 0.58).round() as isize;

    for y in arch_top..=arch_bot.min(rows.saturating_sub(1)) {
        for &x in &[left, right] {
            if x >= 0 && (x as usize) < cols {
                let cell = &mut grid[y * cols + x as usize];
                cell.ch = '│';
                cell.fg = (
                    (70.0 * fade) as u8,
                    (45.0 * fade) as u8,
                    (30.0 * fade) as u8,
                );
                cell.bold = true;
            }
        }
    }
    if arch_top < rows {
        for x in left..=right {
            if x >= 0 && (x as usize) < cols {
                let cell = &mut grid[arch_top * cols + x as usize];
                cell.ch = '═';
                cell.fg = (
                    (90.0 * fade) as u8,
                    (60.0 * fade) as u8,
                    (40.0 * fade) as u8,
                );
            }
        }
    }
    let floor_y = arch_bot.min(rows.saturating_sub(1));
    for x in left..=right {
        if x >= 0 && (x as usize) < cols {
            let cell = &mut grid[floor_y * cols + x as usize];
            let u = (x as f32 - fire_cx) / fire_w.max(1.0);
            let center = (1.0 - u.abs() * 1.6).clamp(0.0, 1.0);
            let flicker =
                0.7 + 0.3 * ((time * 5.0 + x as f32 * 0.4).sin() * 0.5 + 0.5) + coal_boost * 0.5;
            let heat = center * coal_pulse * flicker;
            cell.ch = if heat > 0.55 {
                '█'
            } else if heat > 0.3 {
                '▓'
            } else {
                '▀'
            };
            cell.fg = (
                ((40.0 + 200.0 * heat) * fade).min(255.0) as u8,
                ((18.0 + 90.0 * heat) * fade).min(255.0) as u8,
                ((8.0 + 15.0 * heat) * fade).min(255.0) as u8,
            );
            cell.bg = (
                ((20.0 + 80.0 * heat) * fade).min(255.0) as u8,
                ((8.0 + 25.0 * heat) * fade).min(255.0) as u8,
                ((4.0) * fade) as u8,
            );
            cell.bold = heat > 0.45;
        }
    }
    if floor_y > 0 {
        let cy = floor_y - 1;
        for x in (left + 1)..right {
            if x >= 0 && (x as usize) < cols {
                let cell = &mut grid[cy * cols + x as usize];
                let u = (x as f32 - fire_cx) / fire_w.max(1.0);
                let center = (1.0 - u.abs() * 1.8).clamp(0.0, 1.0);
                let flicker = 0.65 + 0.35 * ((time * 7.0 + x as f32 * 0.6).sin() * 0.5 + 0.5);
                let heat = center * coal_pulse * flicker;
                if heat > 0.2 {
                    cell.ch = if heat > 0.6 {
                        '█'
                    } else if heat > 0.38 {
                        '▓'
                    } else {
                        '░'
                    };
                    cell.fg = (
                        ((60.0 + 195.0 * heat) * fade).min(255.0) as u8,
                        ((25.0 + 85.0 * heat) * fade).min(255.0) as u8,
                        ((10.0 + 12.0 * heat) * fade).min(255.0) as u8,
                    );
                    cell.bold = heat > 0.5;
                }
            }
        }
    }

    Some(arch_top)
}
