use super::*;
use crate::runner::Screensaver;
use crate::runner::hsl_to_rgb;
use crate::runner::{LcgRng, TerminalCell};
use std::time::Duration;

#[test]
fn test_glyphs_new() {
    let g = Glyphs::new();
    assert!(g.drops.is_empty());
    assert!(!g.char_pool.is_empty());
}

#[test]
fn test_glyphs_init() {
    let mut g = Glyphs::new();
    g.update(Duration::from_millis(16), 80, 24);
    assert_eq!(g.last_cols, 80);
    assert_eq!(g.last_rows, 24);
    assert!(!g.drops.is_empty());
}

#[test]
fn test_glyphs_update_and_draw() {
    let mut g = Glyphs::new();
    g.init(80, 24);
    g.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    g.draw(&mut grid, 80, 24);
    // Since digital rain falls over time, checking that cells have non-default contents
    let drawn_count = grid.iter().filter(|c| c.ch != '\0').count();
    assert!(drawn_count > 0, "No glyphs drawn in the grid");
}

#[test]
fn test_lcg_rng_math() {
    let mut rng = LcgRng::new(42);
    // check next_f32 bounds [0.0, 1.0)
    for _ in 0..100 {
        let f = rng.next_f32();
        assert!((0.0..1.0).contains(&f));
    }
    // check next_range bounds
    for _ in 0..100 {
        let val = rng.next_range(1.5, 5.5);
        assert!((1.5..5.5).contains(&val));
    }
    // check next_usize bounds
    for _ in 0..100 {
        let u = rng.next_usize(10);
        assert!(u < 10);
    }
    // check next_bool
    let mut true_count = 0;
    for _ in 0..1000 {
        if rng.next_bool(0.5) {
            true_count += 1;
        }
    }
    assert!(true_count > 300 && true_count < 700); // Statistical range check
}

#[test]
fn test_hsl_to_rgb() {
    // Red (0, 1, 0.5) -> (255, 0, 0)
    let rgb = hsl_to_rgb(0.0, 1.0, 0.5);
    assert_eq!(rgb, (255, 0, 0));

    // Green (120, 1, 0.5) -> (0, 255, 0)
    let rgb = hsl_to_rgb(120.0, 1.0, 0.5);
    assert_eq!(rgb, (0, 255, 0));

    // Blue (240, 1, 0.5) -> (0, 0, 255)
    let rgb = hsl_to_rgb(240.0, 1.0, 0.5);
    assert_eq!(rgb, (0, 0, 255));
}

#[test]
fn test_calculate_glyph_color() {
    let accent = (100, 200, 50);

    // Head (k=0) near layer is full accent intensity before depth scale
    let c0 = physics::calculate_glyph_color(accent, 0, 10, 1);
    assert_eq!(c0, accent);

    // Far layer head is dimmer
    let c0_far = physics::calculate_glyph_color(accent, 0, 10, 0);
    assert!(c0_far.0 < c0.0);

    // Tail tip is much dimmer than head
    let c_tail = physics::calculate_glyph_color(accent, 9, 10, 1);
    assert!(c_tail.0 < c0.0 / 2);
    assert!(c_tail.1 < c0.1 / 2);
}

#[test]
fn test_update_drop_bounds() {
    let mut rng = LcgRng::new(12345);
    let mut drop = physics::spawn_drop(5, 20, 50, &mut rng);

    // Speed should be positive and bounded by far/near ranges
    assert!(drop.speed >= 5.0 && drop.speed <= 32.0);
    // Length in [4, 20)
    assert!(drop.length >= 4 && drop.length < 20);
    // X coordinate remains unchanged
    assert_eq!(drop.x, 5.0);
    assert!(drop.layer <= 1);

    // Let's force it off-screen and update it to trigger respawn
    drop.y = 20.0 + (drop.length as f32) + 1.0;
    physics::update_drop(&mut drop, 0.1, 20, 50, 0.0, &mut rng);

    // Check that it respawned above screen
    assert!(drop.y < 0.0);
}

#[test]
fn test_simulation_run() {
    let mut g = Glyphs::new();
    g.update(Duration::from_millis(16), 80, 24);
    assert!(!g.drops.is_empty());

    let initial_y: Vec<f32> = g.drops.iter().map(|d| d.y).collect();

    // Run update 500 times
    for _ in 0..500 {
        g.update(Duration::from_millis(16), 80, 24);
    }

    // Check that they moved (their Y should not be identical to initial Y)
    let current_y: Vec<f32> = g.drops.iter().map(|d| d.y).collect();
    assert_ne!(initial_y, current_y);

    // Let's make sure none of them have NaN or are stuck
    for d in &g.drops {
        assert!(!d.y.is_nan());
        assert!(d.speed > 0.0);
    }
}

#[test]
fn update_clamps_huge_dt_after_resume() {
    // A suspend/resume (or long hitch) can deliver a multi-minute dt.
    // update() must clamp the simulation step (<=0.1s + first-frame init)
    // rather than teleporting particles or exhausting rockets/timers.
    let mut saver = Glyphs::new();
    saver.update(std::time::Duration::from_secs(300), 80, 24);
    assert!(
        saver.time_elapsed < 1.0,
        "time_elapsed advanced by {} on a 300s dt — clamp missing",
        saver.time_elapsed
    );
}
