//! Unit tests for Cosmos screensaver.

use super::*;
use crate::runner::Screensaver;
use crate::runner::{LcgRng, TerminalCell};
use std::time::Duration;

#[test]
fn test_cosmos_new() {
    let cosmos = Cosmos::new();
    assert_eq!(cosmos.state, UniverseState::Darkness);
    assert_eq!(cosmos.particles.len(), 0);
    assert_eq!(cosmos.seeds.len(), 0);
}

#[test]
fn test_cosmos_update_and_draw() {
    let mut cosmos = Cosmos::new();
    cosmos.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    cosmos.draw(&mut grid, 80, 24);
    assert_eq!(cosmos.last_cols, 80);
    assert_eq!(cosmos.last_rows, 24);
}

#[test]
fn test_coordinate_conversion() {
    let universe_cx = 40.0;
    let universe_cy = 12.0;
    let screen_cx = 40.0;
    let screen_cy = 12.0;
    let zoom = physics::default_zoom(80, 24);

    let test_points = vec![(40.0, 12.0), (10.0, 5.0), (80.0, 24.0), (0.0, 0.0)];
    let round_trip_tol = 0.51 / zoom + 0.01;

    for (ux, uy) in test_points {
        let (sx, sy) =
            physics::to_screen_fast(ux, uy, universe_cx, universe_cy, screen_cx, screen_cy, zoom);
        let rux = universe_cx + (sx as f32 - screen_cx) / zoom;
        let ruy = universe_cy + (sy as f32 - screen_cy) / zoom;
        assert!((ux - rux).abs() < round_trip_tol, "ux {ux} rux {rux}");
        assert!((uy - ruy).abs() < round_trip_tol, "uy {uy} ruy {ruy}");
    }
}

#[test]
fn test_logo_coordinate_round_trip() {
    let universe_cx = 105.0;
    let universe_cy = 28.5;
    let screen_cx = 105.0;
    let screen_cy = 28.5;
    let zoom = 0.58;

    for (gx, gy) in [(60.0, 20.0), (150.0, 40.0), (105.0, 28.5)] {
        let (lp_x, lp_y) = physics::screen_to_logo_universe(
            gx,
            gy,
            universe_cx,
            universe_cy,
            screen_cx,
            screen_cy,
        );
        let (sx, sy) = physics::logo_to_screen_fast(
            lp_x,
            lp_y,
            universe_cx,
            universe_cy,
            screen_cx,
            screen_cy,
        );
        assert!((gx - sx as f32).abs() < 0.51, "gx {gx} sx {sx}");
        assert!((gy - sy as f32).abs() < 0.51, "gy {gy} sy {sy}");

        let (px, py) =
            physics::logo_to_particle_universe(lp_x, lp_y, universe_cx, universe_cy, zoom);
        let (rsx, rsy) =
            physics::to_screen_fast(px, py, universe_cx, universe_cy, screen_cx, screen_cy, zoom);
        assert!((gx - rsx as f32).abs() < 0.51);
        assert!((gy - rsy as f32).abs() < 0.51);
    }
}

#[test]
fn test_lcg_rng() {
    let mut rng = LcgRng::new(42);
    let val1 = rng.next_f32();
    assert!((0.0..1.0).contains(&val1));

    let mut rng2 = LcgRng::new(42);
    let val2 = rng2.next_f32();
    assert_eq!(val1, val2);

    for _ in 0..100 {
        let r = rng.next_range(-5.0, 5.0);
        assert!((-5.0..5.0).contains(&r));
    }
}

#[test]
fn test_hsl_rgb_conversions() {
    use crate::runner::{hsl_to_rgb, rgb_to_hsl};
    let (r, g, b) = hsl_to_rgb(0.0, 1.0, 0.5);
    assert_eq!((r, g, b), (255, 0, 0));
    let (h, s, l) = rgb_to_hsl(r, g, b);
    assert!((h - 0.0).abs() < 1.0);
    assert!((s - 1.0).abs() < 0.01);
    assert!((l - 0.5).abs() < 0.01);
}

#[test]
fn trim_particles_keeps_highest_energy() {
    // Regression: the trim sort was ascending, which culled the hot/fast
    // particles it was meant to preserve.
    let mut eff = Cosmos::new();
    eff.on_battery = true;
    eff.quality_scale = 0.2; // budget = max(580*0.2*0.55, 120) = 120
    for i in 0..130usize {
        let hot = i >= 120;
        eff.particles.push(Particle {
            x: 0.0,
            y: 0.0,
            vx: if hot { 100.0 } else { 0.01 },
            vy: 0.0,
            mass: 1.0,
            color: (255, 255, 255),
            ch: '*',
            history: Vec::new(),
        });
    }
    physics::particle_cap::trim_particles(&mut eff);
    assert_eq!(eff.particles.len(), 120);
    let hot = eff.particles.iter().filter(|p| p.vx.abs() > 50.0).count();
    assert_eq!(hot, 10, "highest-energy particles must survive the trim");
}

/// Regression: the particle budget used to be dead code. `trim_particles` was
/// correct but only ever called from a test, while `merges` and
/// `accretion_helpers` pushed particles with no cap at all — so a long idle
/// run grew `particles` without bound. `update_frame_time` is now the
/// per-frame choke point that calls it.
#[test]
fn update_frame_time_enforces_particle_budget() {
    let mut eff = Cosmos::new();
    eff.on_battery = true;
    eff.quality_scale = 0.2; // budget = max(580*0.2*0.55, 120) = 120
    for i in 0..500usize {
        let hot = i >= 480;
        eff.particles.push(Particle {
            x: 0.0,
            y: 0.0,
            vx: if hot { 100.0 } else { 0.01 },
            vy: 0.0,
            mass: 1.0,
            color: (255, 255, 255),
            ch: '*',
            history: Vec::new(),
        });
    }
    assert!(
        eff.particles.len() > physics::particle_cap::particle_budget(&eff),
        "precondition: well over budget before the frame runs"
    );

    physics::update::update_frame_time(&mut eff, Duration::from_millis(16));

    assert!(
        eff.particles.len() <= physics::particle_cap::particle_budget(&eff),
        "update_frame_time must enforce the particle budget: {} > {}",
        eff.particles.len(),
        physics::particle_cap::particle_budget(&eff)
    );
    assert_eq!(eff.particles.len(), 120);
}

#[test]
fn ignition_consumes_cluster_and_spawns_seed() {
    // 50 particles stacked at one point: every pick sees the same dense
    // clump, so a seeded rng will ignite within a bounded number of tries.
    let mut eff = Cosmos::new();
    eff.rng = LcgRng::new(42);
    eff.state_timer = 2.0;
    eff.universe_cx = 40.0;
    eff.universe_cy = 12.0;
    for _ in 0..50 {
        eff.particles.push(Particle {
            x: 10.0,
            y: 10.0,
            vx: 0.0,
            vy: 0.0,
            mass: 1.0,
            color: (200, 200, 200),
            ch: '*',
            history: Vec::new(),
        });
    }
    for _ in 0..500 {
        if !eff.seeds.is_empty() {
            break;
        }
        physics::ignition::handle_nebular_stellar_ignition(&mut eff, 1.0);
    }
    assert_eq!(eff.seeds.len(), 1, "seeded ignition never fired");
    // The 50-particle clump was consumed; 15 spark shards were emitted.
    assert_eq!(eff.particles.len(), 15);
    assert_eq!(eff.seeds[0].color, (200, 200, 200));
}
