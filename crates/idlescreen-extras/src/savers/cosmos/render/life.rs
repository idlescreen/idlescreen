use super::big_bang_shell::draw_big_bang_shell;
use super::black_hole_disk::bh_radius_from_mass;
use super::black_hole_disk::draw_black_hole_disk;
use super::grav_wave::draw_grav_wave;
use super::particles::draw_particles_and_trails;
use super::seeds::draw_seeds;
use crate::cosmos::Cosmos;
use crate::cosmos::physics::logo_to_screen_fast;
use crate::cosmos::physics::to_screen_fast;
use crate::cosmos::types::UniverseState;
use crate::runner::TerminalCell;

fn draw_logo_pixels(
    effect: &Cosmos,
    grid: &mut [TerminalCell],
    cols: usize,
    rows: usize,
    accent: (u8, u8, u8),
    accent_dim: f32,
) {
    for lp in &effect.logo_pixels {
        if !lp.active {
            continue;
        }
        let (lx, ly) = logo_to_screen_fast(
            lp.x,
            lp.y,
            effect.universe_cx,
            effect.universe_cy,
            effect.screen_cx,
            effect.screen_cy,
        );
        if lx >= 0 && lx < cols as i32 && ly >= 0 && ly < rows as i32 {
            let idx = ly as usize * cols + lx as usize;
            let t = lp.exc;
            let r =
                ((accent.0 as f32 * accent_dim) * (1.0 - t) + 255.0 * t).clamp(0.0, 255.0) as u8;
            let g =
                ((accent.1 as f32 * accent_dim) * (1.0 - t) + 255.0 * t).clamp(0.0, 255.0) as u8;
            let b =
                ((accent.2 as f32 * accent_dim) * (1.0 - t) + 255.0 * t).clamp(0.0, 255.0) as u8;
            grid[idx] = TerminalCell {
                ch: lp.ch,
                fg: (r, g, b),
                bg: (0, 0, 0),
                bold: t > 0.35,
            };
        }
    }
}

fn apply_brightness(grid: &mut [TerminalCell], mul: f32) {
    let m = mul.clamp(0.0, 1.0);
    if m >= 0.999 {
        return;
    }
    for cell in grid.iter_mut() {
        cell.fg = (
            (cell.fg.0 as f32 * m) as u8,
            (cell.fg.1 as f32 * m) as u8,
            (cell.fg.2 as f32 * m) as u8,
        );
    }
}

pub fn draw_life(effect: &Cosmos, grid: &mut [TerminalCell], cols: usize, rows: usize) {
    if cols == 0 || rows == 0 {
        return;
    }

    let accent = effect.cached_accent;
    // Soft phase enter + global intro (especially darkness → bang).
    let phase_mul = {
        let sf = effect.state_fade.clamp(0.0, 1.0);
        // Smoothstep for gentler phase blends.
        let s = sf * sf * (3.0 - 2.0 * sf);
        0.35 + 0.65 * s
    };
    let intro = effect.intro_fade.clamp(0.0, 1.0);
    grid.fill(TerminalCell::default());

    match effect.state {
        UniverseState::Darkness => {
            // Pre-bang hush: almost empty, slowly lifting with intro/state fade.
            draw_particles_and_trails(effect, grid, cols, rows, phase_mul * 0.55);
        }
        UniverseState::BigBang => {
            draw_particles_and_trails(effect, grid, cols, rows, phase_mul);
            draw_logo_pixels(effect, grid, cols, rows, accent, 0.28 * phase_mul);

            if effect.state_timer < 1.85 {
                draw_big_bang_shell(effect, grid, cols, rows);
            }
        }
        UniverseState::Expansion | UniverseState::Accretion => {
            draw_particles_and_trails(effect, grid, cols, rows, phase_mul);
            draw_seeds(effect, grid, cols, rows);
            draw_logo_pixels(effect, grid, cols, rows, accent, 0.28 * phase_mul);
        }
        UniverseState::Singularity => {
            draw_particles_and_trails(effect, grid, cols, rows, phase_mul);
            draw_logo_pixels(effect, grid, cols, rows, accent, 0.20 * phase_mul);

            let bh_univ = if effect.seeds.is_empty() {
                (effect.universe_cx, effect.universe_cy)
            } else {
                (effect.seeds[0].x, effect.seeds[0].y)
            };
            let (bh_x, bh_y) = to_screen_fast(
                bh_univ.0,
                bh_univ.1,
                effect.universe_cx,
                effect.universe_cy,
                effect.screen_cx,
                effect.screen_cy,
                effect.zoom,
            );
            let fade_in = if effect.seeds.is_empty() {
                1.0
            } else {
                (effect.seeds[0].birth_timer / 1.5).min(1.0)
            };
            let (color, mass) = if effect.seeds.is_empty() {
                ((130, 50, 240), 30.0)
            } else {
                (effect.seeds[0].color, effect.seeds[0].mass)
            };
            draw_black_hole_disk(
                effect, grid, cols, rows, bh_x, bh_y, fade_in, color, mass, 10.0,
            );
        }
        UniverseState::Collapse => {
            let collapse_dim = (1.0 - effect.state_timer / 3.33).max(0.0) * phase_mul;
            draw_particles_and_trails(effect, grid, cols, rows, collapse_dim);
            draw_logo_pixels(effect, grid, cols, rows, accent, 0.15 * collapse_dim);

            let bh_univ = if effect.seeds.is_empty() {
                (effect.universe_cx, effect.universe_cy)
            } else {
                (effect.seeds[0].x, effect.seeds[0].y)
            };
            let (bh_x, bh_y) = to_screen_fast(
                bh_univ.0,
                bh_univ.1,
                effect.universe_cx,
                effect.universe_cy,
                effect.screen_cx,
                effect.screen_cy,
                effect.zoom,
            );

            let base_r = if effect.seeds.is_empty() {
                5.0
            } else {
                bh_radius_from_mass(effect.seeds[0].mass)
            };
            let r_universe = (base_r - effect.state_timer * 1.5).max(0.0);
            if r_universe > 0.1 {
                let fade_in = (r_universe / base_r).min(1.0);
                let (color, mass) = if effect.seeds.is_empty() {
                    ((130, 50, 240), 30.0)
                } else {
                    (effect.seeds[0].color, effect.seeds[0].mass)
                };
                draw_black_hole_disk(
                    effect, grid, cols, rows, bh_x, bh_y, fade_in, color, mass, 10.0,
                );
            }

            if bh_x >= 0 && bh_x < cols as i32 && bh_y >= 0 && bh_y < rows as i32 {
                let idx = bh_y as usize * cols + bh_x as usize;
                let pulse = (effect.time_elapsed * 18.0).sin();
                let ch = if pulse > 0.0 { '█' } else { '☼' };
                grid[idx] = TerminalCell {
                    ch,
                    fg: (255, 255, 255),
                    bg: (0, 0, 0),
                    bold: true,
                };
            }
        }
    }

    if effect.grav_wave_timer > 0.0 {
        draw_grav_wave(effect, grid, cols, rows);
    }

    apply_brightness(grid, intro);
}
