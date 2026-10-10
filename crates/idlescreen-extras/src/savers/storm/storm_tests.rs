//! Unit tests for Storm screensaver.

use super::*;
use crate::runner::Screensaver;
use crate::runner::TerminalCell;
use std::time::Duration;

#[test]
fn test_storm_creation() {
    let storm = Storm::new();
    assert_eq!(storm.last_cols, 0);
    assert_eq!(storm.last_rows, 0);
}

#[test]
fn test_storm_update_and_draw() {
    let mut storm = Storm::new();
    storm.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    storm.draw(&mut grid, 80, 24);
    assert_eq!(storm.last_cols, 80);
    assert_eq!(storm.last_rows, 24);
}

#[test]
fn storm_draws_nonempty_after_warmup() {
    let mut storm = Storm::new();
    storm.init(80, 24);
    for _ in 0..90 {
        storm.update(Duration::from_millis(16), 80, 24);
    }
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    storm.draw(&mut grid, 80, 24);
    assert!(
        grid.iter().any(|c| c.ch != ' ' && c.ch != '\0'),
        "storm should paint after warmup"
    );
}

#[test]
fn storm_resize_updates_dims() {
    let mut storm = Storm::new();
    storm.init(40, 12);
    storm.update(Duration::from_millis(16), 40, 12);
    storm.update(Duration::from_millis(16), 100, 30);
    assert_eq!(storm.last_cols, 100);
    assert_eq!(storm.last_rows, 30);
}

#[test]
fn storm_many_frames_no_panic() {
    let mut storm = Storm::new();
    storm.init(64, 20);
    let mut grid = vec![TerminalCell::default(); 64 * 20];
    for _ in 0..150 {
        storm.update(Duration::from_millis(16), 64, 20);
        storm.draw(&mut grid, 64, 20);
    }
}

#[test]
fn storm_draw_grid_length_matches_dims() {
    let mut storm = Storm::new();
    storm.init(50, 18);
    storm.update(Duration::from_millis(16), 50, 18);
    let mut grid = vec![TerminalCell::default(); 50 * 18];
    storm.draw(&mut grid, 50, 18);
    assert_eq!(grid.len(), 50 * 18);
}

#[test]
fn update_clamps_huge_dt_after_resume() {
    // A suspend/resume (or long hitch) can deliver a multi-minute dt.
    // update() must clamp the simulation step (<=0.1s + first-frame init)
    // rather than teleporting particles or exhausting rockets/timers.
    let mut saver = Storm::new();
    saver.update(std::time::Duration::from_secs(300), 80, 24);
    assert!(
        saver.time_elapsed < 1.0,
        "time_elapsed advanced by {} on a 300s dt — clamp missing",
        saver.time_elapsed
    );
}
