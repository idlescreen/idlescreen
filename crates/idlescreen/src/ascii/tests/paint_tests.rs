// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::draw::place_banner_centered;
use crate::cell::TerminalCell;

#[test]
fn test_place_banner_centered() {
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    let lines = vec![
        "█████".to_string(),
        "█   █".to_string(),
        "█████".to_string(),
    ];
    let (x, y, w, h) = place_banner_centered(&mut grid, 80, 24, &lines, (255, 255, 255), (0, 0, 0));
    assert_eq!(w, 5);
    assert_eq!(h, 3);
    assert_eq!(x, (80 - 5) / 2);
    assert_eq!(y, (24 - 3) / 2);

    let idx = y * 80 + x;
    assert_eq!(grid[idx].ch, '█');
}
