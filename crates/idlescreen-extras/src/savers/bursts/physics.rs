//! Helper functions and core calculations for the bursts screensaver.

use super::types::{ActiveExplosion, Particle, Star};

pub fn calculate_star_illumination(
    sx: f32,
    sy: f32,
    active_explosions: &[ActiveExplosion],
) -> (f32, (u8, u8, u8)) {
    let mut best_intensity = 0.0f32;
    let mut lit_color = (0, 0, 0);

    for exp in active_explosions {
        let dx = (sx - exp.x) * 0.55;
        let dy = sy - exp.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < exp.radius {
            let intensity = (1.0 - dist / exp.radius) * exp.intensity;
            if intensity > best_intensity {
                best_intensity = intensity;
                lit_color = exp.color;
            }
        }
    }
    (best_intensity, lit_color)
}

pub fn calculate_logo_illumination(
    gx: f32,
    gy: f32,
    active_explosions: &[ActiveExplosion],
    default_color: (u8, u8, u8),
) -> (f32, (u8, u8, u8)) {
    let mut best_intensity = 0.0f32;
    let mut lit_color = default_color; // dim default silhouette color

    for exp in active_explosions {
        let dx = (gx - exp.x) * 0.55;
        let dy = gy - exp.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < exp.radius {
            let intensity = (1.0 - dist / exp.radius) * exp.intensity;
            if intensity > best_intensity {
                best_intensity = intensity;
                lit_color = (
                    (exp.color.0 as f32 * intensity + default_color.0 as f32 * (1.0 - intensity))
                        as u8,
                    (exp.color.1 as f32 * intensity + default_color.1 as f32 * (1.0 - intensity))
                        as u8,
                    (exp.color.2 as f32 * intensity + default_color.2 as f32 * (1.0 - intensity))
                        as u8,
                );
            }
        }
    }
    (best_intensity, lit_color)
}

pub fn calculate_window_illumination(
    cx: f32,
    cy: f32,
    active_explosions: &[ActiveExplosion],
) -> (f32, (u8, u8, u8)) {
    let mut best_intensity = 0.0f32;
    let mut glow_color = (0, 0, 0);

    for exp in active_explosions {
        let dx = (cx - exp.x) * 0.55;
        let dy = cy - exp.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let glow_radius = exp.radius * 2.2;
        if dist < glow_radius {
            let intensity = (1.0 - dist / glow_radius) * exp.intensity * 0.95;
            if intensity > best_intensity {
                best_intensity = intensity;
                glow_color = exp.color;
            }
        }
    }
    (best_intensity, glow_color)
}

pub fn update_particles_and_excite_stars(
    particles: &mut [Particle],
    stars: &mut [Star],
    delta: f32,
    cols: usize,
    rows: usize,
) {
    let cols_f = cols as f32;
    let rows_f = rows as f32;
    for p in particles {
        p.x += p.vx * delta;
        p.y += p.vy * delta;

        p.vy += 28.0 * delta; // Gravity
        p.vx *= 1.0 - 0.4 * delta; // Drag
        p.vy *= 1.0 - 0.4 * delta;

        p.life -= delta;

        // Excite background stars (ignore smoke particles)
        if p.color != (100, 100, 100) && p.life > 0.0 {
            for star in stars.iter_mut() {
                let sx = star.x * cols_f;
                let sy = star.y * rows_f;
                let dx = p.x - sx;
                if dx >= 3.0 || dx <= -3.0 {
                    continue;
                }
                let dy = (p.y - sy) * 2.0;
                if dy >= 3.0 || dy <= -3.0 {
                    continue;
                }
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < 9.0 {
                    let dist = dist_sq.sqrt();
                    let force = (1.0 - dist / 3.0) * 1.5;
                    if force > star.excitation {
                        star.excitation = force;
                        star.excited_color = p.color;
                    }
                }
            }
        }
    }
}

pub fn blend_horizontal_flare_color(fade: u8, flare_color: (u8, u8, u8)) -> (u8, u8, u8) {
    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
    let fg_b = (fade.saturating_add(45)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
    (fg_r, fg_g, fg_b)
}

pub fn blend_vertical_flare_color(fade: u8, flare_color: (u8, u8, u8)) -> (u8, u8, u8) {
    let fg_r = fade.saturating_add((flare_color.0 as f32 * 0.8) as u8);
    let fg_g = ((fade as f32 * 0.75) as u8).saturating_add((flare_color.1 as f32 * 0.8) as u8);
    let fg_b = (fade.saturating_add(30)).saturating_add((flare_color.2 as f32 * 0.8) as u8);
    (fg_r, fg_g, fg_b)
}

pub fn blend_explosion_flare_h_color(
    current_fg: (u8, u8, u8),
    fade: u8,
    color: (u8, u8, u8),
) -> (u8, u8, u8) {
    let er = color.0;
    let eg = color.1;
    let eb = color.2;
    (
        current_fg
            .0
            .saturating_add((er as f32 * (fade as f32 / 255.0)) as u8),
        current_fg
            .1
            .saturating_add((eg as f32 * (fade as f32 / 255.0)) as u8),
        current_fg.2.saturating_add(
            (eb as f32 * (fade as f32 / 255.0) + 40.0 * (fade as f32 / 255.0)) as u8,
        ),
    )
}

pub fn calculate_star_color_and_sparkle(
    time_elapsed: f32,
    star: &Star,
    lit_color: (u8, u8, u8),
    best_intensity: f32,
) -> ((u8, u8, u8), f32) {
    let sparkle_base = f32::midpoint((time_elapsed * 2.5 + star.phase).sin(), 1.0);
    let sparkle = (sparkle_base + star.excitation).min(2.0);
    let brightness = (sparkle_base * 0.35 + star.excitation * 0.65).min(1.0);

    let mut r = (110.0 + brightness * 70.0) as u8;
    let mut g = (120.0 + brightness * 75.0) as u8;
    let mut b = (140.0 + brightness * 80.0) as u8;

    if best_intensity > 0.05 {
        let blend = (best_intensity * 0.8).min(1.0);
        r = (r as f32 * (1.0 - blend) + lit_color.0 as f32 * blend).min(255.0) as u8;
        g = (g as f32 * (1.0 - blend) + lit_color.1 as f32 * blend).min(255.0) as u8;
        b = (b as f32 * (1.0 - blend) + lit_color.2 as f32 * blend).min(255.0) as u8;
    }

    if star.excitation > 0.05 {
        let blend = (star.excitation * 0.7).min(1.0);
        r = (r as f32 * (1.0 - blend) + star.excited_color.0 as f32 * blend).min(255.0) as u8;
        g = (g as f32 * (1.0 - blend) + star.excited_color.1 as f32 * blend).min(255.0) as u8;
        b = (b as f32 * (1.0 - blend) + star.excited_color.2 as f32 * blend).min(255.0) as u8;
    }

    ((r, g, b), sparkle)
}

pub fn calculate_window_glow(
    is_lit_window: bool,
    base_fg: (u8, u8, u8),
    glow_color: (u8, u8, u8),
    best_intensity: f32,
) -> (char, (u8, u8, u8)) {
    if is_lit_window {
        let fg = (
            (base_fg.0 as f32 * (1.0 - best_intensity) + glow_color.0 as f32 * best_intensity)
                .min(255.0) as u8,
            (base_fg.1 as f32 * (1.0 - best_intensity) + glow_color.1 as f32 * best_intensity)
                .min(255.0) as u8,
            (base_fg.2 as f32 * (1.0 - best_intensity) + glow_color.2 as f32 * best_intensity)
                .min(255.0) as u8,
        );
        ('■', fg)
    } else {
        let fg = (
            (glow_color.0 as f32 * best_intensity * 0.38) as u8,
            (glow_color.1 as f32 * best_intensity * 0.38) as u8,
            (glow_color.2 as f32 * best_intensity * 0.38) as u8,
        );
        ('■', fg)
    }
}

#[cfg(test)]
#[path = "physics_tests.rs"]
mod tests;
