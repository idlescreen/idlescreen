// SPDX-License-Identifier: Apache-2.0

use super::types::{EnemyShip, ExplosionParticle, GoodShip, RadarJet};
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
pub fn draw_entities(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    enemies: &[EnemyShip],
    defenders: &[GoodShip],
    jets: &[RadarJet],
    particles: &[ExplosionParticle],
    sweep_angle: f32,
    fade: f32,
    accent: (u8, u8, u8),
    friend: (u8, u8, u8),
) {
    for enemy in enemies.iter().filter(|e| e.active) {
        if enemy.visibility > 0.02 {
            let c = enemy.x as usize;
            let r = enemy.y as usize;
            if c < cols && r < rows {
                let idx = r * cols + c;
                if idx < grid.len() {
                    let phosphor = ((enemy.visibility / 3.0).clamp(0.0, 1.0).sqrt()) * fade;
                    let factor = phosphor.clamp(0.12, 1.0);
                    if enemy.target_type == 3 {
                        if c > 0 && c < cols - 1 {
                            grid[idx - 1].ch = '[';
                            grid[idx - 1].fg = scale_rgb((255, 50, 50), factor);
                            grid[idx].ch =
                                std::char::from_digit(enemy.health as u32, 10).unwrap_or('B');
                            grid[idx].fg = scale_rgb((255, 255, 255), factor);
                            grid[idx + 1].ch = ']';
                            grid[idx + 1].fg = scale_rgb((255, 50, 50), factor);
                        }
                    } else if enemy.target_type == 2 {
                        let flash = ((sweep_angle * 12.0) as u32).is_multiple_of(2);
                        if enemy.altitude >= 1.8 {
                            grid[idx].ch = '☢';
                            grid[idx].fg = scale_rgb(
                                if flash {
                                    (255, 255, 255)
                                } else {
                                    (255, 50, 50)
                                },
                                factor,
                            );
                        } else if enemy.altitude >= 1.0 {
                            grid[idx].ch = '☣';
                            grid[idx].fg = scale_rgb((255, 120, 0), factor);
                        } else {
                            grid[idx].ch = '*';
                            grid[idx].fg = scale_rgb((255, 50, 50), factor);
                        }
                    } else if enemy.target_type == 1 {
                        grid[idx].ch = 'M';
                        grid[idx].fg = (
                            (255.0 * factor) as u8,
                            (50.0 * factor) as u8,
                            (50.0 * factor) as u8,
                        );
                    } else {
                        grid[idx].ch = 'x';
                        grid[idx].fg = (
                            (255.0 * factor) as u8,
                            (120.0 * factor) as u8,
                            (40.0 * factor) as u8,
                        );
                    }
                }
            }
        }
    }

    for def in defenders.iter().filter(|d| d.active) {
        let c = def.x as usize;
        let r = def.y as usize;
        if c < cols && r < rows {
            let idx = r * cols + c;
            if idx < grid.len() {
                grid[idx].ch = if def.altitude >= 0.8 { '^' } else { '·' };
                grid[idx].fg = friend;
            }
        }
    }

    for jet in jets.iter().filter(|j| j.active) {
        let c = jet.x as usize;
        let r = jet.y as usize;
        if c < cols && r < rows {
            let idx = r * cols + c;
            if idx < grid.len() {
                grid[idx].ch = if jet.altitude >= 1.6 {
                    jet.heading_ch
                } else {
                    match jet.heading_ch {
                        '▶' => '▸',
                        '◀' => '◂',
                        '▲' => '▴',
                        '▼' => '▾',
                        _ => '·',
                    }
                };
                grid[idx].fg = scale_rgb(
                    (
                        (255.0 * 0.85 + accent.0 as f32 * 0.15) as u8,
                        (200.0 * 0.85 + accent.1 as f32 * 0.15) as u8,
                        (50.0 * 0.7 + accent.2 as f32 * 0.3) as u8,
                    ),
                    fade,
                );
            }
        }
    }

    for p in particles {
        let c = p.x as usize;
        let r = p.y as usize;
        if c < cols && r < rows {
            let idx = r * cols + c;
            if idx < grid.len() {
                let intensity = (p.life / p.max_life).clamp(0.0, 1.0) * fade;
                grid[idx].ch = if intensity > 0.6 { '*' } else { '.' };
                if p.max_life > 1.0 {
                    grid[idx].fg = (
                        (255.0 * intensity) as u8,
                        (180.0 * intensity) as u8,
                        (40.0 * intensity) as u8,
                    );
                } else {
                    grid[idx].fg = (
                        (200.0 * intensity) as u8,
                        (60.0 * intensity) as u8,
                        (60.0 * intensity) as u8,
                    );
                }
            }
        }
    }

    let cx = cols / 2;
    let cy = rows / 2;
    if cx < cols && cy < rows {
        let idx = cy * cols + cx;
        if idx < grid.len() {
            grid[idx].ch = '⌖';
            grid[idx].fg = scale_rgb((255, 255, 255), fade);
        }
    }
}
