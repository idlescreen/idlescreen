// SPDX-License-Identifier: Apache-2.0

use super::blips;
use super::draw_hud::{draw_datalinks, draw_hud, draw_laser_beam, draw_lock_line};
use super::types::{EnemyShip, ExplosionParticle, GoodShip, LaserBeam, RadarJet};
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
pub fn draw_simulation(
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    sweep_angle: f32,
    enemies: &[EnemyShip],
    defenders: &[GoodShip],
    jets: &[RadarJet],
    lasers: &[LaserBeam],
    particles: &[ExplosionParticle],
    shield_health: f32,
    palette: &ScreenPalette,
    intro_fade: f32,
    accent: (u8, u8, u8),
) {
    let cx = cols as f32 / 2.0;
    let cy = rows as f32 / 2.0;
    let fade = intro_fade.clamp(0.0, 1.0);

    let faint_accent = scale_rgb(accent, 0.15 * fade);
    let friend = scale_rgb(
        (
            ((60.0 + accent.0 as f32 * 0.25) as u8),
            ((255.0 * 0.85 + accent.1 as f32 * 0.15) as u8),
            ((200.0 * 0.7 + accent.2 as f32 * 0.3) as u8),
        ),
        fade,
    );

    let afterglow_offsets: [f32; 4] = [0.12, 0.28, 0.48, 0.72];
    let afterglow_strength: [f32; 4] = [0.45, 0.28, 0.16, 0.08];

    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            if idx >= grid.len() {
                continue;
            }

            let dx = c as f32 - cx;
            let dy = (r as f32 - cy) * 2.0;
            let dist = (dx * dx + dy * dy).sqrt();

            let mut cell_angle = dy.atan2(dx);
            if cell_angle < 0.0 {
                cell_angle += std::f32::consts::TAU;
            }

            let is_base_ring =
                (dist - 10.0).abs() < 0.5 || (dist - 20.0).abs() < 0.5 || (dist - 30.0).abs() < 0.5;

            let is_shield_ring = (dist - 3.0).abs() < 0.5;

            let mut diff = sweep_angle - cell_angle;
            if diff < 0.0 {
                diff += std::f32::consts::TAU;
            }
            let max_trail = std::f32::consts::FRAC_PI_2;
            let mut main_intensity = if diff < max_trail {
                1.0 - (diff / max_trail)
            } else {
                0.0
            };

            for (off, strength) in afterglow_offsets.iter().zip(afterglow_strength.iter()) {
                let mut gdiff = diff - off;
                if gdiff < 0.0 {
                    gdiff += std::f32::consts::TAU;
                }
                if gdiff < 0.06 {
                    let ridge = (1.0 - gdiff / 0.06) * strength;
                    if ridge > main_intensity {
                        main_intensity = ridge;
                    }
                }
            }
            main_intensity *= fade;

            let mut max_jet_intensity = 0.0f32;
            let mut in_jet_range = false;

            for jet in jets.iter().filter(|j| j.active) {
                // Cheap bbox reject before the per-cell sqrt+atan2 —
                // jdist <= 8.0 requires |jdx| <= 8.0 and |jdy| <= 8.0.
                let jdx = c as f32 - jet.x;
                if jdx.abs() > 8.0 {
                    continue;
                }
                let jdy = (r as f32 - jet.y) * 2.0;
                if jdy.abs() > 8.0 {
                    continue;
                }
                let jdist = (jdx * jdx + jdy * jdy).sqrt();

                if jdist <= 8.0 {
                    in_jet_range = true;
                    let mut jangle = jdy.atan2(jdx);
                    if jangle < 0.0 {
                        jangle += std::f32::consts::TAU;
                    }
                    let mut jdiff = jet.sweep_angle - jangle;
                    if jdiff < 0.0 {
                        jdiff += std::f32::consts::TAU;
                    }
                    if jdiff < max_trail {
                        let jit = (1.0 - (jdiff / max_trail)) * fade;
                        if jit > max_jet_intensity {
                            max_jet_intensity = jit;
                        }
                    }
                }
            }

            let scan = if r % 2 == 1 { 0.82 } else { 1.0 };

            if is_shield_ring {
                grid[idx].ch = '·';
                let factor = (shield_health / 100.0).clamp(0.2, 1.0) * fade;
                grid[idx].fg = scale_rgb(friend, factor);
            } else if main_intensity > 0.0 {
                grid[idx].ch = if is_base_ring { 'o' } else { '.' };
                let s = main_intensity * scan;
                grid[idx].fg = (
                    (accent.0 as f32 * s) as u8,
                    (accent.1 as f32 * s) as u8,
                    (accent.2 as f32 * s) as u8,
                );
            } else if max_jet_intensity > 0.0 {
                grid[idx].ch = '.';
                let s = max_jet_intensity * scan;
                grid[idx].fg = (
                    (accent.0 as f32 * 0.35 * s) as u8,
                    (accent.1 as f32 * 0.55 * s) as u8,
                    (accent.2 as f32 * 0.85 * s + 40.0 * s) as u8,
                );
            } else if is_base_ring {
                grid[idx].ch = '·';
                grid[idx].fg = scale_rgb(faint_accent, scan);
            } else if in_jet_range && (c % 2 == 0) && (r % 2 == 0) {
                grid[idx].ch = '·';
                grid[idx].fg = (
                    (15.0 * fade * scan) as u8,
                    (25.0 * fade * scan) as u8,
                    (35.0 * fade * scan) as u8,
                );
            } else if r % 3 == 0 && c % 5 == 0 {
                grid[idx].ch = '·';
                grid[idx].fg = ((4.0 * fade) as u8, (6.0 * fade) as u8, (8.0 * fade) as u8);
            } else {
                grid[idx].ch = ' ';
                grid[idx].fg = (0, 0, 0);
            }
            grid[idx].bg = (0, 0, 0);
            grid[idx].bold = false;
        }
    }

    draw_datalinks(grid, cols, rows, jets, cx, cy, sweep_angle, fade, accent);

    for jet in jets
        .iter()
        .filter(|j| j.active && j.lock_target_id.is_some())
    {
        if let Some(target_id) = jet.lock_target_id
            && let Some(enemy) = enemies.iter().find(|e| e.active && e.id == target_id)
        {
            let jdx = enemy.x - jet.x;
            let jdy = (enemy.y - jet.y) * 2.0;
            let dist_sq = jdx * jdx + jdy * jdy;
            if dist_sq <= 64.0 {
                draw_lock_line(
                    grid,
                    cols,
                    rows,
                    jet.x,
                    jet.y,
                    enemy.x,
                    enemy.y,
                    jet.lock_timer,
                    fade,
                );
            }
        }
    }

    for laser in lasers {
        draw_laser_beam(
            grid,
            cols,
            rows,
            laser.from_x,
            laser.from_y,
            laser.to_x,
            laser.to_y,
            laser.life,
            laser.max_life,
            fade,
        );
    }

    blips::draw_entities(
        grid,
        cols,
        rows,
        enemies,
        defenders,
        jets,
        particles,
        sweep_angle,
        fade,
        accent,
        friend,
    );

    draw_hud(
        grid,
        cols,
        rows,
        enemies,
        defenders,
        shield_health,
        palette,
        sweep_angle,
        fade,
    );
}
