use super::*;
use crate::runner::Screensaver;
use crate::runner::{LcgRng, TerminalCell, hsl_to_rgb, lerp, percentage, rgb_to_hsl};
use std::time::Duration;

#[test]
fn test_chaos_new() {
    let chaos = Chaos::new();
    assert_eq!(chaos.phase, Phase::Assembled);
    assert_eq!(chaos.particles.len(), 0);
}

#[test]
fn test_chaos_update_and_draw() {
    let mut chaos = Chaos::new();
    // Use sys_refresh_timer = -1000.0 to prevent slow sys_info calls
    chaos.sys_refresh_timer = -1000.0;
    chaos.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    chaos.draw(&mut grid, 80, 24);
    // Should initialize and have stars/particles set up
    assert!(!chaos.stars.is_empty());
    assert!(!chaos.particles.is_empty());
}

#[test]
fn test_math_lcg_rng() {
    let mut rng = LcgRng::new(42);
    // Test that consecutive numbers are deterministic but changing
    let a = rng.next_u64();
    let b = rng.next_u64();
    assert_ne!(a, b);

    // Test next_f32 bounds
    for _ in 0..100 {
        let f = rng.next_f32();
        assert!((0.0..=1.0).contains(&f));
    }

    // Test next_range bounds
    for _ in 0..100 {
        let val = rng.next_range(-5.0, 10.0);
        assert!((-5.0..=10.0).contains(&val));
    }

    // Test next_usize bounds
    for _ in 0..100 {
        let u = rng.next_usize(5);
        assert!(u < 5);
    }

    // Test next_bool distribution properties (0.0 should be false, 1.0 should be true)
    assert!(!rng.next_bool(0.0));
    assert!(rng.next_bool(1.0));
}

#[test]
fn test_math_lerp_and_percentage() {
    // Test percentage
    assert_eq!(percentage(50, 100), 50.0);
    assert_eq!(percentage(0, 100), 0.0);
    assert_eq!(percentage(50, 0), 0.0);

    // Test lerp
    assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
    assert_eq!(lerp(-5.0, 5.0, 0.25), -2.5);
    // Clamp test
    assert_eq!(lerp(0.0, 10.0, 1.5), 10.0);
    assert_eq!(lerp(0.0, 10.0, -0.5), 0.0);
}

#[test]
fn test_math_color_conversions() {
    // HSL <-> RGB round-trip for some colors
    let (r, g, b) = (255, 0, 0);
    let (h, s, l) = rgb_to_hsl(r, g, b);
    assert!((h - 0.0).abs() < 1.0 || (h - 360.0).abs() < 1.0);
    assert!((s - 1.0).abs() < 0.01);
    assert!((l - 0.5).abs() < 0.01);

    let (r_out, g_out, b_out) = hsl_to_rgb(h, s, l);
    assert_eq!(r_out, r);
    assert_eq!(g_out, g);
    assert_eq!(b_out, b);
}

#[test]
fn test_explosion_type_rotation() {
    let mut exp = ExplosionType::Supernova;
    let mut count = 0;
    loop {
        exp = exp.next();
        count += 1;
        if exp == ExplosionType::Supernova {
            break;
        }
        assert!(count < 10, "ExplosionType did not loop properly");
    }
    assert_eq!(count, 7);
}

#[test]
fn test_chaos_phase_transitions() {
    let mut chaos = Chaos::new();
    chaos.sys_refresh_timer = -1000.0;

    // Setup and trigger size change to initialize particles/stars
    chaos.update(Duration::from_millis(16), 80, 24);
    assert_eq!(chaos.phase, Phase::Assembled);

    // Force phase_timer past assembled limit to trigger explosion phase
    chaos.phase_timer = 15.0;
    chaos.update(Duration::from_millis(16), 80, 24);
    assert_eq!(chaos.phase, Phase::Exploding); // Assembled -> Exploding on update

    // Next update advances to Chaos
    chaos.update(Duration::from_millis(16), 80, 24);
    assert_eq!(chaos.phase, Phase::Chaos); // Exploding -> Chaos
}

#[test]
fn update_clamps_huge_dt_after_resume() {
    // A suspend/resume (or long hitch) can deliver a multi-minute dt.
    // update() must clamp the simulation step (<=0.1s + first-frame init)
    // rather than teleporting particles or exhausting rockets/timers.
    let mut saver = Chaos::new();
    saver.update(std::time::Duration::from_secs(300), 80, 24);
    assert!(
        saver.time_elapsed < 1.0,
        "time_elapsed advanced by {} on a 300s dt — clamp missing",
        saver.time_elapsed
    );
}
