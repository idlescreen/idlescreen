//! Terminal cell drawing logic for Hearth screensaver.

use super::types::{Ember, Log, Smoke, Tongue};
use crate::runner::TerminalCell;

pub fn ember_rgb(heat: f32, a: f32) -> (f32, f32, f32) {
    let r = (250.0 * heat + 70.0 * (1.0 - heat)) * a;
    let g = (140.0 * heat * heat + 15.0 * (1.0 - heat)) * a;
    let b = (35.0 * heat * heat * heat + 10.0) * a;
    (r, g, b)
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_impl(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    embers: &[Ember],
    smoke: &[Smoke],
    logs: &[Log],
    tongues: &[Tongue],
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    mantel_y: usize,
    intro_fade: f32,
    time: f32,
    coal_boost: f32,
    accent: (u8, u8, u8),
    logo_text: &str,
    kernel: &str,
) {
    if cols == 0 || rows == 0 || grid.is_empty() {
        return;
    }
    let fade = intro_fade.clamp(0.0, 1.0);
    let (ar, ag, ab) = accent;
    let fire_pulse = 0.88 + 0.12 * (time * 3.2).sin();
    let coal_pulse = fire_pulse + coal_boost * 0.45;

    let arch_top = match super::background::draw_background(
        grid, cols, rows, fire_cx, fire_y, fire_w, mantel_y, fade, fire_pulse, coal_pulse,
        coal_boost, time, accent,
    ) {
        Some(at) => at,
        None => return,
    };

    for log in logs {
        let flicker = 0.7 + 0.3 * (log.phase.sin() * 0.5 + 0.5);
        for dx in 0..log.w {
            let x = log.x + dx;
            if x >= cols || log.y >= rows {
                continue;
            }
            let cell = &mut grid[log.y * cols + x];
            cell.ch = if dx == 0 || dx + 1 == log.w {
                '━'
            } else {
                '═'
            };
            cell.fg = (
                ((50.0 + 40.0 * flicker + 30.0 * coal_boost) * fade).min(255.0) as u8,
                ((28.0 + 15.0 * flicker) * fade) as u8,
                (12.0 * fade) as u8,
            );
            cell.bold = true;
        }
    }

    let opening = (fire_y - arch_top as f32).max(4.0);
    let base_h = (opening * 0.78 * fire_pulse).max(5.0);
    for tongue in tongues {
        let tongue_h = (base_h * tongue.height_scale) as usize;
        let lean = tongue.phase.sin() * 2.2 + tongue.offset * 0.5;
        let base_x = fire_cx + tongue.offset * fire_w * 0.38;
        for dy in 0..tongue_h {
            let y = (fire_y as usize).saturating_sub(1 + dy);
            if y <= arch_top || y >= rows {
                continue;
            }
            let t = 1.0 - dy as f32 / tongue_h as f32;
            let wobble = (tongue.phase * 1.3 + dy as f32 * 0.55).sin() * (1.0 + (1.0 - t) * 1.8);
            let half = (fire_w * 0.14 * tongue.width_scale * t * fire_pulse).max(0.8);
            let cx = base_x + lean * (1.0 - t) + wobble;
            let span = (half * 2.0).ceil() as usize;
            for xi in 0..=span {
                let x = (cx - half + xi as f32).round() as isize;
                if x < 0 || x >= cols as isize {
                    continue;
                }
                let cell = &mut grid[y * cols + x as usize];
                let edge = (xi as f32 - half).abs() / half.max(0.1);
                if edge > 1.05 {
                    continue;
                }
                let hot = t * (1.0 - edge * 0.55);
                let ch = if hot > 0.75 {
                    '█'
                } else if hot > 0.5 {
                    '▓'
                } else if hot > 0.28 {
                    '░'
                } else {
                    '\''
                };
                let rank = match ch {
                    '█' => 3,
                    '▓' => 2,
                    '░' => 1,
                    _ => 0,
                };
                let existing = match cell.ch {
                    '█' => 3,
                    '▓' => 2,
                    '░' => 1,
                    _ => 0,
                };
                if rank < existing {
                    continue;
                }
                cell.ch = ch;
                let tip = (1.0 - t).clamp(0.0, 1.0);
                let r = 180.0 + 75.0 * hot + ar as f32 * tip * 0.25;
                let g = 40.0 + 140.0 * hot * fire_pulse + ag as f32 * tip * 0.15;
                let b = 8.0 + 20.0 * (1.0 - hot) + ab as f32 * tip * 0.2;
                cell.fg = (
                    (r * fade).min(255.0) as u8,
                    (g * fade).min(255.0) as u8,
                    (b * fade).min(255.0) as u8,
                );
                cell.bold = hot > 0.55;
            }
        }
    }

    super::fx::draw_fx(
        grid, cols, rows, embers, smoke, fire_cx, fire_y, fire_w, arch_top, fade, time, accent,
        logo_text, kernel,
    );
}
