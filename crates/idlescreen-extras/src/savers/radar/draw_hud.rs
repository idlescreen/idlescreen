// SPDX-License-Identifier: MIT

use super::types::{EnemyShip, GoodShip, RadarJet};
use crate::runner::ScreenPalette;
use crate::runner::TerminalCell;

fn scale_rgb(c: (u8, u8, u8), s: f32) -> (u8, u8, u8) {
    (
        (c.0 as f32 * s).min(255.0) as u8,
        (c.1 as f32 * s).min(255.0) as u8,
        (c.2 as f32 * s).min(255.0) as u8,
    )
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_lock_line(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    jet_x: f32,
    jet_y: f32,
    enemy_x: f32,
    enemy_y: f32,
    lock_timer: f32,
    fade: f32,
) {
    let steps = 18;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let lx = jet_x + (enemy_x - jet_x) * t;
        let ly = jet_y + (enemy_y - jet_y) * t;
        let c = lx as usize;
        let r = ly as usize;
        if c < cols && r < rows {
            let idx = r * cols + c;
            if idx < grid.len() {
                grid[idx].ch = if s % 2 == 0 { '+' } else { '·' };
                let flash = if ((lock_timer * 10.0) as u32).is_multiple_of(2) {
                    255.0
                } else {
                    150.0
                };
                grid[idx].fg = ((flash * fade) as u8, (128.0 * fade) as u8, 0);
            }
        }
    }
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_laser_beam(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    life: f32,
    max_life: f32,
    fade: f32,
) {
    let steps = 40;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let lx = from_x + (to_x - from_x) * t;
        let ly = from_y + (to_y - from_y) * t;
        let c = lx as usize;
        let r = ly as usize;
        if c < cols && r < rows {
            let idx = r * cols + c;
            if idx < grid.len() {
                grid[idx].ch = if s % 3 == 0 {
                    '%'
                } else if s % 2 == 0 {
                    '#'
                } else {
                    '@'
                };
                let mix = (life / max_life * 10.0) as u32 % 2;
                let base = if mix == 0 {
                    (255, 255, 255)
                } else {
                    (255, 0, 255)
                };
                grid[idx].fg = scale_rgb(base, fade);
            }
        }
    }
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_datalinks(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    jets: &[RadarJet],
    cx: f32,
    cy: f32,
    sweep_angle: f32,
    fade: f32,
    accent: (u8, u8, u8),
) {
    for jet in jets.iter().filter(|j| j.active) {
        let steps = 15;
        let is_locked = jet.lock_target_id.is_some();
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let lx = cx + (jet.x - cx) * t;
            let ly = cy + (jet.y - cy) * t;
            let c = lx as usize;
            let r = ly as usize;
            if c < cols && r < rows {
                let idx = r * cols + c;
                if idx < grid.len() && grid[idx].ch == ' ' {
                    grid[idx].ch = '·';
                    if is_locked {
                        let pulse = if (s + (sweep_angle * 10.0) as usize).is_multiple_of(3) {
                            255.0
                        } else {
                            100.0
                        };
                        grid[idx].fg = ((pulse * fade) as u8, (100.0 * fade) as u8, 0);
                    } else {
                        grid[idx].fg = scale_rgb(
                            (
                                (30.0 + accent.0 as f32 * 0.1) as u8,
                                (45.0 + accent.1 as f32 * 0.1) as u8,
                                (50.0 + accent.2 as f32 * 0.1) as u8,
                            ),
                            fade,
                        );
                    }
                }
            }
        }
    }
}

// physics function with many positional inputs (positions, velocities, parameters); refactor to RenderContext struct tracked for Sprint-03 housekeeping.
#[allow(clippy::too_many_arguments)]
pub fn draw_hud(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    enemies: &[EnemyShip],
    defenders: &[GoodShip],
    shield_health: f32,
    palette: &ScreenPalette,
    sweep_angle: f32,
    fade: f32,
) {
    let accent = scale_rgb(palette.accent, fade);
    let label = " [ SECTOR DEFENSE ACTIVE ] ";
    if cols > label.len() + 2 && rows > 2 {
        for (i, ch) in label.chars().enumerate() {
            let idx = cols + 2 + i;
            if idx < grid.len() {
                grid[idx].ch = ch;
                grid[idx].fg = accent;
            }
        }
    }

    let nuke_active = enemies.iter().any(|e| e.active && e.target_type == 2);
    let boss_active = enemies.iter().any(|e| e.active && e.target_type == 3);

    if (nuke_active || boss_active) && rows > 4 {
        let flash = ((sweep_angle * 6.0) as u32).is_multiple_of(2);
        let warning_text = if nuke_active {
            "⚠️ WARNING: BALISTIC NUCLEAR PAYLOAD ACQUIRED ⚠️"
        } else {
            "⚠️ WARNING: DREADNOUGHT CLASS SIGNATURE DETECTED ⚠️"
        };

        if flash && cols > warning_text.len() + 4 {
            let start_col = (cols - warning_text.len()) / 2;
            let cy = rows as f32 / 2.0;
            let warning_row = (cy as usize).saturating_sub(4);
            let row_offset = warning_row * cols;
            for (i, ch) in warning_text.chars().enumerate() {
                let idx = row_offset + start_col + i;
                if idx < grid.len() {
                    grid[idx].ch = ch;
                    let col = if nuke_active {
                        (255, 50, 50)
                    } else {
                        (255, 160, 0)
                    };
                    grid[idx].fg = scale_rgb(col, fade);
                }
            }
        }
    }

    let detected_count = enemies
        .iter()
        .filter(|e| e.active && e.visibility > 0.0)
        .count();
    let def_count = defenders.iter().filter(|d| d.active).count();
    let stats = format!(
        " SHIELD: {}% | HOSTILES: {} | DRONES: {} ",
        shield_health as u32, detected_count, def_count
    );

    if cols > stats.len() + 4 && rows > 3 {
        let start_col = cols - stats.len() - 2;
        let row_idx = (rows - 2) * cols;
        for (i, ch) in stats.chars().enumerate() {
            let idx = row_idx + start_col + i;
            if idx < grid.len() {
                grid[idx].ch = ch;
                grid[idx].fg = if shield_health < 40.0 || nuke_active {
                    scale_rgb((255, 60, 60), fade)
                } else {
                    accent
                };
            }
        }
    }
}
