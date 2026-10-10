//! Unit tests for Hearth screensaver.

use super::Hearth;
use crate::runner::Screensaver;
use crate::runner::TerminalCell;
use std::time::Duration;

#[test]
fn hearth_runs() {
    let mut h = Hearth::new();
    h.init(80, 24);
    for _ in 0..60 {
        h.update(Duration::from_millis(40), 80, 24);
    }
    assert!(!h.embers.is_empty(), "embers should keep spawning");
    assert!(!h.tongues.is_empty());
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    h.draw(&mut grid, 80, 24);
    assert!(grid.iter().any(|c| c.ch != ' ' && c.ch != '\0'));
}

#[test]
fn hearth_init_then_resize_keeps_drawing() {
    let mut h = Hearth::new();
    h.init(40, 12);
    h.update(Duration::from_millis(40), 40, 12);
    h.update(Duration::from_millis(40), 100, 30);
    let mut grid = vec![TerminalCell::default(); 100 * 30];
    h.draw(&mut grid, 100, 30);
    assert_eq!(grid.len(), 100 * 30);
    assert!(grid.iter().any(|c| c.ch != ' ' && c.ch != '\0'));
}

#[test]
fn hearth_many_frames_no_panic() {
    let mut h = Hearth::new();
    h.init(60, 20);
    let mut grid = vec![TerminalCell::default(); 60 * 20];
    for _ in 0..200 {
        h.update(Duration::from_millis(16), 60, 20);
        h.draw(&mut grid, 60, 20);
    }
    assert!(!h.embers.is_empty() || !h.tongues.is_empty());
}

#[test]
fn hearth_zero_dt_is_safe() {
    let mut h = Hearth::new();
    h.init(32, 12);
    h.update(Duration::ZERO, 32, 12);
    let mut grid = vec![TerminalCell::default(); 32 * 12];
    h.draw(&mut grid, 32, 12);
}

#[test]
fn hearth_draw_grid_length_matches_dims() {
    let mut h = Hearth::new();
    h.init(50, 18);
    h.update(Duration::from_millis(40), 50, 18);
    let mut grid = vec![TerminalCell::default(); 50 * 18];
    h.draw(&mut grid, 50, 18);
    assert_eq!(grid.len(), 50 * 18);
}
