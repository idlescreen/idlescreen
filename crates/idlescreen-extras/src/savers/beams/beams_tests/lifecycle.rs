use super::super::*;
use crate::runner::Screensaver;
use crate::runner::TerminalCell;
use std::time::Duration;

#[test]
fn test_beams_new() {
    let b = Beams::new();
    assert!(!b.spotlights.is_empty());
    assert_eq!(b.last_cols, 0);
    assert_eq!(b.last_rows, 0);
}

#[test]
fn test_beams_init() {
    let mut b = Beams::new();
    b.update(Duration::from_millis(16), 80, 24);
    assert_eq!(b.last_cols, 80);
    assert_eq!(b.last_rows, 24);
    assert!(!b.stars.is_empty());
    assert!(!b.particles.is_empty());
}

#[test]
fn test_beams_update_and_draw() {
    let mut b = Beams::new();
    b.init(80, 24);
    b.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    b.draw(&mut grid, 80, 24);
    let drawn_count = grid.iter().filter(|c| c.ch != '\0').count();
    assert!(drawn_count > 0, "No beams/particles drawn in the grid");
}
