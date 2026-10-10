// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::draw::paint_transition;
use crate::cell::TerminalCell;

#[test]
fn test_paint_transition_bounds() {
    let mut from = vec![TerminalCell::default(); 80 * 24];
    let mut to = vec![TerminalCell::default(); 80 * 24];
    let mut out = vec![TerminalCell::default(); 80 * 24];

    from[0].ch = 'A';
    to[0].ch = 'B';

    paint_transition(&from, &to, &mut out, 0.0, 80, 24);
    assert_eq!(out[0].ch, 'A');

    paint_transition(&from, &to, &mut out, 1.0, 80, 24);
    assert_eq!(out[0].ch, 'B');

    // Partial transition
    paint_transition(&from, &to, &mut out, 0.5, 80, 24);
    assert!(out[0].ch == 'A' || out[0].ch == 'B');
}
