//! Unit tests for physics and math helper functions.

use super::*;
use crate::bursts::types::{ActiveExplosion, Particle, Star};

#[test]
fn test_calculate_star_illumination_no_explosion() {
    let (intensity, color) = calculate_star_illumination(10.0, 10.0, &[]);
    assert_eq!(intensity, 0.0);
    assert_eq!(color, (0, 0, 0));
}

#[test]
fn test_calculate_star_illumination_with_explosion() {
    let explosions = vec![ActiveExplosion {
        x: 10.0,
        y: 10.0,
        radius: 5.0,
        color: (255, 100, 50),
        intensity: 1.0,
    }];

    // Exact center
    let (intensity, color) = calculate_star_illumination(10.0, 10.0, &explosions);
    assert_eq!(intensity, 1.0);
    assert_eq!(color, (255, 100, 50));

    // Outer edge
    let (intensity, color) = calculate_star_illumination(14.0, 10.0, &explosions);
    // dx = 4.0 * 0.55 = 2.2, dy = 0.0, dist = 2.2
    // intensity = (1.0 - 2.2/5.0) * 1.0 = 0.56
    assert!(intensity > 0.0 && intensity < 1.0);
    assert!(color.0 > 0 && color.1 > 0);
}

#[test]
fn test_calculate_logo_illumination() {
    let explosions = vec![ActiveExplosion {
        x: 20.0,
        y: 20.0,
        radius: 10.0,
        color: (100, 200, 255),
        intensity: 0.8,
    }];

    // Outside radius
    let (intensity, color) = calculate_logo_illumination(50.0, 50.0, &explosions, (35, 15, 50));
    assert_eq!(intensity, 0.0);
    assert_eq!(color, (35, 15, 50)); // Default silhouette color

    // Inside radius
    let (intensity, color) = calculate_logo_illumination(20.0, 20.0, &explosions, (35, 15, 50));
    assert!(intensity > 0.0);
    // At center, intensity should be exp.intensity = 0.8
    assert_eq!(intensity, 0.8);
    // color.0 = 100 * 0.8 + 35 * 0.2 = 80 + 7 = 87
    assert_eq!(color.0, 87);
}

#[test]
fn test_blend_flare_colors() {
    let h_color = blend_horizontal_flare_color(100, (50, 100, 150));
    assert_eq!(h_color.0, 100 + 40); // 100 + 50*0.8 = 140
    assert_eq!(h_color.1, 75 + 80); // 100*0.75 + 100*0.8 = 155
    assert_eq!(h_color.2, 255); // saturating addition (over 255 -> 255)
    assert_eq!(h_color.2, 255);

    let v_color = blend_vertical_flare_color(100, (50, 100, 150));
    assert_eq!(v_color.0, 140);
    assert_eq!(v_color.1, 155);
    assert_eq!(v_color.2, 250); // 100 + 30 + 120 = 250
}

#[test]
fn test_calculate_star_color_and_sparkle() {
    let star = Star {
        x: 0.5,
        y: 0.5,
        phase: 0.0,
        ch: '.',
        excitation: 0.5,
        excited_color: (200, 100, 50),
    };

    let ((r, g, b), sparkle) = calculate_star_color_and_sparkle(1.0, &star, (100, 100, 100), 0.0);
    assert!(sparkle >= 0.5);
    // Since excitation is 0.5 (>0.05), blending should occur
    assert!(r > 0);
    assert!(g > 0);
    assert!(b > 0);
}

#[test]
fn test_calculate_window_glow() {
    // Unlit window with glow
    let (ch, fg) = calculate_window_glow(false, (0, 0, 0), (255, 100, 50), 0.5);
    assert_eq!(ch, '■');
    // fg should be glow_color * best_intensity * 0.38
    assert_eq!(fg.0, (255.0 * 0.5 * 0.38) as u8); // 48

    // Lit window with glow
    let (ch, fg) = calculate_window_glow(true, (255, 200, 100), (0, 0, 255), 0.8);
    assert_eq!(ch, '■');
    // fg should be base_fg * 0.2 + glow_color * 0.8
    assert_eq!(fg.2, (100.0 * 0.2 + 255.0 * 0.8) as u8); // 20 + 204 = 224
}

#[test]
fn test_update_particles_and_excite_stars() {
    let mut particles = vec![Particle {
        x: 10.0,
        y: 10.0,
        vx: 2.0,
        vy: -2.0,
        color: (255, 255, 255),
        ch: '*',
        life: 1.0,
        max_life: 1.0,
    }];

    let mut stars = vec![Star {
        x: 10.0 / 80.0,
        y: 10.0 / 24.0,
        phase: 0.0,
        ch: '.',
        excitation: 0.0,
        excited_color: (0, 0, 0),
    }];

    update_particles_and_excite_stars(&mut particles, &mut stars, 0.1, 80, 24);

    // Particle should move
    assert!(particles[0].x > 10.0);
    // Gravity should apply (vy increases)
    assert!(particles[0].vy > -2.0);
    // Star should be excited since particle is close
    assert!(stars[0].excitation > 0.0);
    assert_eq!(stars[0].excited_color, (255, 255, 255));
}
