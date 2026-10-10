// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::Ascii;
use super::super::effects::EffectKind;
use super::super::state::{BrandTarget, TextMode};
use crate::cell::{LcgRng, TerminalCell};
use std::time::Duration;

const COLS: usize = 80;
const ROWS: usize = 24;

#[test]
fn every_effect_survives_running_without_panicking() {
    let mut saver = Ascii::new();
    saver.init(COLS, ROWS);
    let mut grid = vec![TerminalCell::default(); COLS * ROWS];
    for &kind in &EffectKind::ALL {
        saver.pin_effect(kind);
        for _ in 0..10 {
            saver.update(Duration::from_millis(16), COLS, ROWS);
            saver.draw(&mut grid, COLS, ROWS);
        }
    }
}

#[test]
fn render_is_deterministic_for_a_fixed_seed() {
    let render = |seed: u64| {
        let mut saver = Ascii::new_with_seed(seed);
        saver.init(COLS, ROWS);
        saver.pin_effect(EffectKind::Decrypt);
        let mut grid = vec![TerminalCell::default(); COLS * ROWS];
        for _ in 0..100 {
            saver.update(Duration::from_millis(16), COLS, ROWS);
            saver.draw(&mut grid, COLS, ROWS);
        }
        grid.iter().map(|c| c.ch).collect::<Vec<char>>()
    };
    assert_eq!(render(1234), render(1234));
}

#[test]
fn resize_rebuilds_the_grid_and_art() {
    let mut saver = Ascii::new();
    saver.init(COLS, ROWS);
    let mut grid = vec![TerminalCell::default(); COLS * ROWS];
    saver.update(Duration::from_millis(16), COLS, ROWS);
    saver.draw(&mut grid, COLS, ROWS);

    let (wide, tall) = (120, 40);
    saver.update(Duration::from_millis(16), wide, tall);
    let mut grid_wide = vec![TerminalCell::default(); wide * tall];
    saver.draw(&mut grid_wide, wide, tall);
    assert_eq!(saver.last_cols, wide);
    assert_eq!(saver.last_rows, tall);

    let (small_w, small_h) = (20, 10);
    saver.update(Duration::from_millis(16), small_w, small_h);
    let mut grid_small = vec![TerminalCell::default(); small_w * small_h];
    saver.draw(&mut grid_small, small_w, small_h);
    assert_eq!(saver.last_cols, small_w);
    assert_eq!(saver.last_rows, small_h);
}

#[test]
fn tri_rotation_mutation_cycles_without_immediate_repeat() {
    let mut rng = LcgRng::new(42);
    let mut last = BrandTarget::Os;
    for _ in 0..100 {
        let next = BrandTarget::random_excluding(&mut rng, Some(last));
        assert_ne!(next, last, "tri-rotation repeated consecutively");
        last = next;
    }
}

#[test]
fn custom_mode_locks_text() {
    let mut saver = Ascii::new().with_mode(TextMode::Custom, Some("CUSTOM_BRAND".to_string()));
    saver.init(COLS, ROWS);
    assert_eq!(saver.rotation.active_text(), "CUSTOM_BRAND");
}

#[test]
fn text_mode_parse_and_labels() {
    assert_eq!(TextMode::parse("os"), TextMode::Os);
    assert_eq!(TextMode::parse("Host OS"), TextMode::Os);
    assert_eq!(TextMode::parse("de"), TextMode::De);
    assert_eq!(TextMode::parse("kernel"), TextMode::Kernel);
    assert_eq!(TextMode::parse("Linux Kernel"), TextMode::Kernel);
    assert_eq!(TextMode::parse("custom"), TextMode::Custom);
    assert_eq!(TextMode::parse("cycle"), TextMode::Cycle);

    assert_eq!(TextMode::Os.as_str(), "os");
    assert_eq!(TextMode::De.as_str(), "de");
    assert_eq!(TextMode::Kernel.as_str(), "kernel");
    assert_eq!(TextMode::Cycle.as_str(), "cycle");
    assert_eq!(TextMode::Custom.as_str(), "custom");
}

#[test]
fn non_custom_mode_ignores_custom_text() {
    let mut saver = Ascii::new().with_mode(TextMode::Os, Some("IGNORED_CUSTOM".to_string()));
    saver.init(COLS, ROWS);
    assert_eq!(saver.rotation.active_text(), saver.rotation.os_name);
}
