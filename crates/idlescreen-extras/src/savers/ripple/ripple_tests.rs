//! Unit tests for Ripple screensaver.

use super::Ripple;
use super::physics::wave_displace;
use super::types::Ring;
use crate::runner::Screensaver;
use crate::runner::TerminalCell;
use std::time::Duration;

#[test]
fn ripple_runs() {
    let mut r = Ripple::new();
    r.init(80, 24);
    for _ in 0..80 {
        r.update(Duration::from_millis(40), 80, 24);
    }
    r.impact(40.0, 12.0, 0.9, 80, 24);
    assert!(!r.rings.is_empty());
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    r.draw(&mut grid, 80, 24);
    assert!(grid.iter().any(|c| c.ch != ' ' && c.ch != '\0'));
}

#[test]
fn wave_displace_moves_near_front() {
    let rings = [Ring {
        x: 10.0,
        y: 10.0,
        r: 5.0,
        max_r: 20.0,
        life: 1.0,
        strength: 1.0,
        age: 0.2,
    }];
    let (dx, _dy, hit) = wave_displace(15.0, 10.0, &rings);
    assert!(hit > 0.3, "front should register hit, got {hit}");
    assert!(dx.abs() > 0.1, "should push along radial, dx={dx}");
    let (_dx2, _dy2, hit2) = wave_displace(40.0, 40.0, &rings);
    assert!(hit2 < 0.05);
}

#[test]
fn weather_cycles() {
    let mut r = Ripple::new();
    r.init(60, 20);
    r.weather_timer = 0.0;
    r.update(Duration::from_millis(50), 60, 20);
    assert!(r.weather_timer > 0.0);
}

#[test]
fn ripple_resize_still_draws() {
    let mut r = Ripple::new();
    r.init(40, 12);
    r.update(Duration::from_millis(40), 40, 12);
    r.update(Duration::from_millis(40), 90, 28);
    r.impact(45.0, 14.0, 1.0, 90, 28);
    let mut grid = vec![TerminalCell::default(); 90 * 28];
    r.draw(&mut grid, 90, 28);
    assert!(grid.iter().any(|c| c.ch != ' ' && c.ch != '\0'));
}

#[test]
fn ripple_many_impacts_bounded_rings() {
    let mut r = Ripple::new();
    r.init(80, 24);
    for i in 0..40 {
        r.impact((i * 3) as f32 % 80.0, (i * 2) as f32 % 24.0, 0.5, 80, 24);
        r.update(Duration::from_millis(40), 80, 24);
    }
    // Rings may expire; must not panic or explode unbounded forever.
    assert!(r.rings.len() < 10_000);
}

#[test]
fn ripple_zero_dt_safe() {
    let mut r = Ripple::new();
    r.init(32, 12);
    r.update(Duration::ZERO, 32, 12);
    let mut grid = vec![TerminalCell::default(); 32 * 12];
    r.draw(&mut grid, 32, 12);
}

#[test]
fn ripple_draw_grid_length_matches_dims() {
    let mut r = Ripple::new();
    r.init(50, 18);
    r.update(Duration::from_millis(40), 50, 18);
    let mut grid = vec![TerminalCell::default(); 50 * 18];
    r.draw(&mut grid, 50, 18);
    assert_eq!(grid.len(), 50 * 18);
}
