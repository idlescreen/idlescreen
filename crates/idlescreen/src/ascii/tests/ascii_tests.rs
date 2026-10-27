// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::Ascii;
use super::super::effects::EffectKind;
use super::super::state::{BrandTarget, TextMode};
use crate::cell::{LcgRng, TerminalCell};
use std::time::Duration;

const COLS: usize = 80;
const ROWS: usize = 24;

fn step(saver: &mut Ascii, frames: usize) {
    let mut grid = vec![TerminalCell::default(); COLS * ROWS];
    for _ in 0..frames {
        saver.update(Duration::from_millis(16), COLS, ROWS);
        saver.draw(&mut grid, COLS, ROWS);
    }
}

#[test]
fn every_effect_survives_running_without_panicking() {
    let mut saver = Ascii::new();
    for &(c, r, steps) in &[(0, 0, 2), (1, 1, 5), (COLS, ROWS, 10)] {
        saver.init(c, r);
        let mut grid = vec![TerminalCell::default(); c * r];
        for &kind in &EffectKind::ALL {
            saver.pin_effect(kind);
            for _ in 0..steps {
                saver.update(Duration::from_millis(16), c, r);
                saver.draw(&mut grid, c, r);
            }
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

#[test]
fn multi_target_scene_handoff_and_single_lock() {
    let mut single = Ascii::new().with_targets(vec!["FEDORA".to_string()]);
    single.init(COLS, ROWS);
    assert_eq!(single.rotation.current_text(), "FEDORA");
    assert_eq!(single.rotation.peek_next_text(), "FEDORA");
    single.rotation.advance_scene();
    assert_eq!(single.rotation.current_text(), "FEDORA");

    let mut multi =
        Ascii::new().with_targets(vec!["fedora linux".to_string(), "gnome mutter".to_string()]);
    multi.init(COLS, ROWS);
    assert_eq!(multi.rotation.current_text(), "FEDORA");
    assert_eq!(multi.rotation.peek_next_text(), "GNOME");

    multi.begin_transition();
    assert!(multi.transition_active);
    assert_eq!(multi.rotation.current_text(), "FEDORA");
    assert_eq!(multi.rotation.peek_next_text(), "GNOME");

    for _ in 0..10 {
        multi.update(Duration::from_millis(100), COLS, ROWS);
    }
    assert!(!multi.transition_active);
    assert_eq!(multi.rotation.current_text(), "GNOME");
    assert_eq!(multi.rotation.peek_next_text(), "FEDORA");

    multi.begin_transition();
    assert!(multi.transition_active);
    for _ in 0..10 {
        multi.update(Duration::from_millis(100), COLS, ROWS);
    }
    assert!(!multi.transition_active);
    assert_eq!(multi.rotation.current_text(), "FEDORA");
    assert_eq!(multi.rotation.peek_next_text(), "GNOME");
}

#[test]
fn decoupled_dwell_fills_the_slot_then_holds_the_last_frame() {
    let mut saver = Ascii::new().with_dwell(2.0);
    saver.init(COLS, ROWS);
    saver.pin_effect(EffectKind::Wipe);
    let mut frames = 0;
    while saver.engine.is_some() && frames < 1000 {
        saver.update(Duration::from_millis(16), COLS, ROWS);
        frames += 1;
    }
    // The scene plays out over its whole slot instead of racing through it.
    let played = frames as f32 * 0.016;
    assert!(
        (played - 2.0).abs() < 0.3,
        "the scene should fill its 2s slot, it played for {played}s"
    );
    assert!(saver.engine.is_none());
    assert!(saver.dwell_left <= 0.0);
    for _ in 0..60 {
        saver.update(Duration::from_millis(16), COLS, ROWS);
    }
    assert!(!saver.transition_active);
}

#[test]
fn effect_dwells_on_settled_frame_before_restarting() {
    let mut saver = Ascii::new().with_dwell(5.0);
    saver.init(COLS, ROWS);
    saver.pin_effect(EffectKind::Wipe);
    let mut grid = vec![TerminalCell::default(); COLS * ROWS];
    for _ in 0..400 {
        saver.update(Duration::from_millis(16), COLS, ROWS);
        saver.draw(&mut grid, COLS, ROWS);
    }
    assert!(saver.engine.is_none());
    assert!(!saver.current_frame.is_empty());
    assert!(grid.iter().any(|c| c.ch != ' '));
}

#[test]
fn smooth_transition_and_resize_without_panics() {
    let mut saver = Ascii::new().with_dwell(5.0);
    saver.init(COLS, ROWS);
    saver.pin_effect(EffectKind::Wipe);
    step(&mut saver, 160);
    saver.begin_transition();
    assert!(saver.transition_active);
    let mut grid = vec![TerminalCell::default(); COLS * ROWS];
    saver.draw(&mut grid, COLS, ROWS);
    assert!(grid.iter().any(|c| c.ch != ' '));

    saver.update(Duration::from_millis(16), 100, 30);
    let mut new_grid = vec![TerminalCell::default(); 100 * 30];
    saver.draw(&mut new_grid, 100, 30);
    assert_eq!(new_grid.len(), 3000);
}

#[test]
fn large_screen_400_percent_rendering_and_effects() {
    let cols = 240;
    let rows = 67;
    let mut saver = Ascii::new();
    saver.init(cols, rows);
    saver.pin_effect(EffectKind::Wipe);
    assert_eq!(saver.last_cols, cols);
    assert_eq!(saver.last_rows, rows);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    for _ in 0..30 {
        saver.update(Duration::from_millis(16), cols, rows);
        saver.draw(&mut grid, cols, rows);
    }
    assert!(grid.iter().any(|c| c.ch != ' '));
}

#[test]
fn multi_target_random_rotation_no_immediate_repeat() {
    let mut rotation =
        super::super::state::TriRotationState::new_with_seed(TextMode::Custom, None, 999);
    rotation.set_targets(vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
        "D".to_string(),
    ]);
    let mut rng = LcgRng::new(12345);

    let mut last_text = rotation.current_text();
    for _ in 0..100 {
        rotation.advance_mutation(&mut rng);
        let cur_text = rotation.current_text();
        assert_ne!(cur_text, last_text, "targets must never immediately repeat");
        last_text = cur_text;
    }
}
