use super::*;
use crate::runner::TerminalCell;
use std::time::Duration;

#[test]
fn test_gnats_new() {
    let gnats = Gnats::new();
    assert_eq!(gnats.fireflies.len(), 0);
    assert_eq!(gnats.attractors.len(), 0);
    assert_eq!(gnats.stars.len(), 0);
    assert_eq!(gnats.kill_sparks.len(), 0);
    assert_eq!(gnats.time_elapsed, 0.0);
}

#[test]
fn test_gnats_update_and_draw() {
    let mut gnats = Gnats::new();
    gnats.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    gnats.draw(&mut grid, 80, 24);
    // Ensure state variables get initialized
    assert_eq!(gnats.last_cols, 80);
    assert_eq!(gnats.last_rows, 24);
    assert!(!gnats.fireflies.is_empty());
    assert!(!gnats.stars.is_empty());
    assert!(!gnats.attractors.is_empty());
}
