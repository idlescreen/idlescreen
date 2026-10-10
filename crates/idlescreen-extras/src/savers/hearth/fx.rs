//! Particle effects, heat shimmer, and caption overlay drawing for Hearth.

use super::draw::ember_rgb;
use super::types::{Ember, Smoke};
use crate::runner::TerminalCell;

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_fx(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    embers: &[Ember],
    smoke: &[Smoke],
    fire_cx: f32,
    fire_y: f32,
    fire_w: f32,
    arch_top: usize,
    fade: f32,
    time: f32,
    accent: (u8, u8, u8),
    logo_text: &str,
    kernel: &str,
) {
    let (ar, ag, ab) = accent;

    for s in smoke {
        let x = s.x as usize;
        let y = s.y as usize;
        if x >= cols || y >= rows || y as f32 > fire_y - 2.0 {
            continue;
        }
        let a = (s.life / s.max_life).clamp(0.0, 1.0) * fade * 0.7;
        if a < 0.05 {
            continue;
        }
        let cell = &mut grid[y * cols + x];
        if cell.ch == '█' || cell.ch == '═' || cell.ch == '│' {
            continue;
        }
        cell.ch = if a > 0.45 { '~' } else { '·' };
        let gray = 50.0 + 40.0 * a;
        cell.fg = ((gray * 0.9) as u8, (gray * 0.9) as u8, gray as u8);
    }

    for y in arch_top.saturating_sub(3)..arch_top {
        if y >= rows {
            break;
        }
        for x in 0..cols {
            let dx = x as f32 - fire_cx;
            if dx.abs() > fire_w * 0.7 {
                continue;
            }
            let phase = time * 4.0 + x as f32 * 0.35 + y as f32 * 0.5;
            if phase.sin() > 0.72 {
                let cell = &mut grid[y * cols + x];
                if cell.ch == ' ' || cell.ch == '·' {
                    cell.ch = if (phase * 2.0).sin() > 0.0 { '~' } else { '·' };
                    cell.fg = (
                        (90.0 * fade) as u8,
                        (50.0 * fade) as u8,
                        (30.0 * fade) as u8,
                    );
                }
            }
        }
    }

    for e in embers {
        let x = e.x as usize;
        let y = e.y as usize;
        if x >= cols || y >= rows {
            continue;
        }
        let life_f = (e.life / e.max_life).clamp(0.0, 1.0);
        let a = life_f * fade;
        if a < 0.04 {
            continue;
        }
        let cell = &mut grid[y * cols + x];
        cell.ch = match e.size {
            2 if e.heat > 0.6 => '*',
            2 => '+',
            _ if e.heat > 0.7 => '°',
            _ if a > 0.4 => '.',
            _ => '·',
        };
        let (r, g, b) = ember_rgb(e.heat, a);
        cell.fg = (r.min(255.0) as u8, g.min(255.0) as u8, b.min(255.0) as u8);
        cell.bold = e.heat > 0.75;
    }

    let caption = if kernel.is_empty() || kernel == "unknown" || kernel == "export" {
        logo_text.to_string()
    } else {
        format!("{logo_text}  ·  {kernel}")
    };
    let chars: Vec<char> = caption.chars().collect();
    let max_w = (cols.saturating_sub(4)).min(40);
    let show: Vec<char> = chars.into_iter().take(max_w).collect();
    if !show.is_empty() {
        let cy = ((arch_top as f32) * 0.45).round() as usize;
        let cy = cy.min(arch_top.saturating_sub(2)).max(1);
        let start = (cols as isize - show.len() as isize) / 2;
        for (i, ch) in show.iter().enumerate() {
            if *ch == ' ' {
                continue;
            }
            let x = start + i as isize;
            if x < 0 || x >= cols as isize {
                continue;
            }
            let xf = x as f32;
            let yf = cy as f32;
            let mut cover = 0.0f32;
            for s in smoke {
                let dx = s.x - xf;
                let dy = s.y - yf;
                let dist = (dx * dx + dy * dy * 1.6).sqrt();
                if dist < 2.8 {
                    let life = (s.life / s.max_life).clamp(0.0, 1.0);
                    cover += (1.0 - dist / 2.8) * life;
                }
            }
            if cover < 0.22 {
                continue;
            }
            let a = (cover * 0.85).clamp(0.0, 1.0) * fade;
            let cell = &mut grid[cy * cols + x as usize];
            cell.ch = *ch;
            cell.fg = (
                ((90.0 + ar as f32 * 0.35) * a).min(255.0) as u8,
                ((75.0 + ag as f32 * 0.3) * a).min(255.0) as u8,
                ((70.0 + ab as f32 * 0.35) * a).min(255.0) as u8,
            );
            cell.bold = cover > 0.7;
        }
    }
}
