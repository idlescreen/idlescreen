use crate::cosmos::Cosmos;
use crate::cosmos::types::UniverseState;
use crate::runner::{get_system_info, query_current_palette};
use std::time::Duration;

use super::accretion::update_accretion;
use super::collapse::update_collapse;
use super::enter_state;
use super::expansion::update_expansion;
use super::history::push_particle_history;
use super::logo::rebuild_logo_pixels;
use super::singularity::update_singularity;

pub fn update_frame_time(eff: &mut Cosmos, dt: Duration) {
    let dt_secs = dt.as_secs_f32();

    if eff.time_elapsed < 2.0 && dt_secs > 0.001 && dt_secs < eff.target_frame_time - 0.001 {
        eff.target_frame_time = dt_secs;
    }

    eff.frame_time_ema = eff.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;

    if eff.time_elapsed > 1.5 {
        let speed_mult = if eff.on_battery { 0.65 } else { 1.0 };
        let delta = dt_secs.min(0.1) * (eff.sim_speed_opt.max(1) as f32) * speed_mult;
        let cells = eff.last_cols.saturating_mul(eff.last_rows);
        let grid_cap = if cells > 10_000 {
            0.65
        } else if cells > 6_000 {
            0.78
        } else {
            1.0
        };

        if eff.frame_time_ema > eff.target_frame_time * 1.12 {
            eff.quality_scale = (eff.quality_scale - 0.20 * delta).max(0.48);
        } else if eff.frame_time_ema < eff.target_frame_time * 1.04 {
            eff.quality_scale = (eff.quality_scale + 0.03 * delta).min(grid_cap);
        }
        eff.quality_scale = eff.quality_scale.min(grid_cap);
    }

    // Enforce the particle budget every frame, after quality_scale has settled
    // so the budget reflects the current frame cost. Merges push 25+ particles
    // per event and accretion adds more, so without this the sim grows without
    // limit over a multi-hour idle run while every frame re-iterates the whole
    // list in `gravity`. `trim_particles` short-circuits when under budget.
    super::particle_cap::trim_particles(eff);
}

pub fn update_life(eff: &mut Cosmos, dt: Duration, cols: usize, rows: usize) {
    let dt_secs = dt.as_secs_f32();
    let speed_mult = if eff.on_battery { 0.65 } else { 1.0 };
    let delta = dt_secs.min(0.1) * (eff.sim_speed_opt.max(1) as f32) * speed_mult;
    eff.time_elapsed += delta;
    eff.state_timer += delta;

    // Intro fade ~0.45s (esp. darkness → bang feel).
    if eff.intro_fade < 1.0 {
        eff.intro_fade = (eff.intro_fade + delta / 0.45).min(1.0);
    }
    // Soft phase blend after each state enter.
    if eff.state_fade < 1.0 {
        eff.state_fade = (eff.state_fade + delta / 0.55).min(1.0);
    }

    for seed in &mut eff.seeds {
        if seed.active {
            seed.birth_timer += delta;
        }
    }

    if eff.grav_wave_timer > 0.0 {
        eff.grav_wave_timer = (eff.grav_wave_timer - delta).max(0.0);
    }

    eff.sys_refresh_timer += delta;
    if eff.sys_refresh_timer >= 1.0 {
        let sys = get_system_info();
        eff.mem_pressure = sys.mem_used_pct / 100.0;
        eff.cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
        eff.on_battery = sys.power_status.contains("Battery");
        eff.cached_accent = query_current_palette().accent;
        eff.sys_refresh_timer = 0.0;
    }

    if cols != eff.last_cols || rows != eff.last_rows {
        eff.last_cols = cols;
        eff.last_rows = rows;
        eff.refresh_screen_cache(cols, rows);
        eff.universe_cx = eff.screen_cx;
        eff.universe_cy = eff.screen_cy;
        eff.zoom = super::default_zoom(cols, rows);
        eff.cached_accent = query_current_palette().accent;
        rebuild_logo_pixels(eff, cols, rows);

        eff.state = UniverseState::Darkness;
        eff.state_timer = 0.0;
        eff.intro_fade = 0.0;
        eff.state_fade = 0.0;
        enter_state(eff, cols, rows);
    }

    // Slightly longer dwell on calm states so transitions feel less abrupt.
    let next_state = match eff.state {
        UniverseState::Darkness => {
            // Hold the void a beat longer before the bang (and under intro).
            if eff.state_timer >= 2.4 + eff.host_bias * 1.5 {
                Some(UniverseState::BigBang)
            } else {
                None
            }
        }
        UniverseState::BigBang => {
            if eff.state_timer >= 1.85 {
                Some(UniverseState::Expansion)
            } else {
                None
            }
        }
        UniverseState::Expansion => {
            let avg_vel_sq = if eff.particles.is_empty() {
                0.0
            } else {
                eff.particles
                    .iter()
                    .map(|p| p.vx * p.vx + p.vy * p.vy)
                    .sum::<f32>()
                    / eff.particles.len() as f32
            };
            let avg_logo_dist_sq = if eff.logo_pixels.is_empty() {
                0.0
            } else {
                eff.logo_pixels
                    .iter()
                    .filter(|lp| lp.active)
                    .map(|lp| {
                        let dx = lp.x - lp.origin_x;
                        let dy = lp.y - lp.origin_y;
                        dx * dx + dy * dy
                    })
                    .sum::<f32>()
                    / eff.logo_pixels.iter().filter(|lp| lp.active).count().max(1) as f32
            };
            if (avg_vel_sq < 256.0 && avg_logo_dist_sq < 0.4225) || eff.state_timer >= 10.0 {
                Some(UniverseState::Accretion)
            } else {
                None
            }
        }
        UniverseState::Accretion => {
            let active_seeds = eff.seeds.iter().filter(|s| s.active).count();
            if active_seeds <= 1 {
                Some(UniverseState::Singularity)
            } else {
                None
            }
        }
        UniverseState::Singularity => {
            if eff.state_timer >= 6.5 {
                Some(UniverseState::Collapse)
            } else {
                None
            }
        }
        UniverseState::Collapse => {
            if eff.particles.is_empty() || eff.state_timer >= 5.0 {
                Some(UniverseState::Darkness)
            } else {
                None
            }
        }
    };

    if let Some(ns) = next_state {
        eff.state = ns;
        eff.state_timer = 0.0;
        // Soft blend into the new phase (draw multiplies by state_fade).
        eff.state_fade = 0.0;
        enter_state(eff, cols, rows);
    }

    match eff.state {
        UniverseState::Darkness => {
            for p in &mut eff.particles {
                p.vx *= 1.0 - (delta * 0.5);
                p.vy *= 1.0 - (delta * 0.5);
                p.x += p.vx * delta;
                p.y += p.vy * delta;
            }
        }
        UniverseState::BigBang => {
            for p in &mut eff.particles {
                p.vx *= 1.0 - (delta * 0.40);
                p.vy *= 1.0 - (delta * 0.40);
                p.x += p.vx * delta;
                p.y += p.vy * delta;

                push_particle_history(&mut p.history, p.x.round() as i32, p.y.round() as i32);
            }

            for lp in &mut eff.logo_pixels {
                if lp.active {
                    lp.vx *= 1.0 - (delta * 0.40);
                    lp.vy *= 1.0 - (delta * 0.40);
                    lp.x += lp.vx * delta;
                    lp.y += lp.vy * delta;
                    lp.exc = (lp.exc - 0.2 * delta).max(0.6);
                }
            }
        }
        UniverseState::Expansion => update_expansion(eff, delta, cols, rows),
        UniverseState::Accretion => update_accretion(eff, delta, cols, rows),
        UniverseState::Singularity => update_singularity(eff, delta, cols, rows),
        UniverseState::Collapse => update_collapse(eff, delta, cols, rows),
    }
}
