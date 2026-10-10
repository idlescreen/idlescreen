// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::Ascii;
use super::super::effects::EffectKind;
use super::super::state::{BrandTarget, TextMode};
use crate::cell::{LcgRng, TerminalCell};
use clap::Parser;
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

#[rustfmt::skip]
const ALL_37_NAMES: &[&str] = &[
    "beams", "binarypath", "blackhole", "bouncyballs", "bubbles", "burn",
    "colorshift", "crumble", "decrypt", "errorcorrect", "expand", "fireworks",
    "highlight", "laseretch", "matrix", "middleout", "orbittingvolley",
    "overflow", "pour", "print", "rain", "randomsequence", "rings",
    "scattered", "slice", "slide", "smoke", "spotlights", "spray",
    "swarm", "sweep", "synthgrid", "thunderstorm", "unstable", "vhstape",
    "waves", "wipe",
];

#[test]
fn all_37_effects_can_be_parsed() {
    assert_eq!(ALL_37_NAMES.len(), 37);
    for name in ALL_37_NAMES {
        let cli = ttfx::cli::Cli::try_parse_from(["ttfx", name]).unwrap();
        assert!(cli.effect.is_some(), "failed to parse effect: {name}");
        let kind = EffectKind::parse(name);
        assert!(kind.is_some(), "EffectKind::parse failed for {name}");
    }
}

#[test]
fn all_37_effects_can_build_and_render_frames() {
    let art_data = "IDLESCREEN\nSCREENSAVER";
    for name in ALL_37_NAMES {
        let cli = ttfx::cli::Cli::try_parse_from(["ttfx", name]).unwrap();
        let mut config = cli.terminal_config();
        let cmd = cli.effect.unwrap();
        let mut effect = cmd.build_effect();
        config.canvas_width = 80;
        config.canvas_height = 24;
        config.ignore_terminal_dimensions = true;
        config.anchor_text = ttfx::engine::canvas::Anchor::C;
        let clock = ttfx::engine::ctx::Clock::virtual_with_frame_rate(60);
        let rng = ttfx::utils::rng::Rng::seeded(42);
        let mut ctx = ttfx::engine::ctx::EngineCtx::new(art_data, config, rng, clock).unwrap();
        effect.build(&mut ctx).unwrap();
        let mut frame_count = 0;
        while effect.next_frame(&mut ctx).is_some() {
            frame_count += 1;
            assert!(
                frame_count <= 5000,
                "Effect {name} exceeded 5000 frames without terminating!"
            );
        }
        assert!(
            frame_count > 0,
            "Effect {name} must produce at least one frame"
        );
    }
}

#[test]
fn decoupled_dwell_animates_until_settled_then_holds_dwell() {
    let mut saver = Ascii::new().with_dwell(2.0);
    saver.init(COLS, ROWS);
    saver.pin_effect(EffectKind::Wipe);
    assert!((saver.anim_left - 120.0).abs() < 0.001);
    while saver.engine.is_some() {
        saver.update(Duration::from_millis(16), COLS, ROWS);
    }
    assert!((saver.dwell_left - 2.0).abs() < 0.05);
    for _ in 0..60 {
        saver.update(Duration::from_millis(16), COLS, ROWS);
    }
    assert!(!saver.transition_active);
    assert!((saver.dwell_left - 1.0).abs() < 0.15);
}

#[test]
fn effect_dwells_on_settled_frame_before_restarting() {
    let mut saver = Ascii::new().with_dwell(5.0);
    saver.init(COLS, ROWS);
    saver.pin_effect(EffectKind::Wipe);
    let mut grid = vec![TerminalCell::default(); COLS * ROWS];
    for _ in 0..160 {
        saver.update(Duration::from_millis(16), COLS, ROWS);
        saver.draw(&mut grid, COLS, ROWS);
    }
    assert!(saver.engine.is_none());
    assert!(!saver.current_frame.is_empty());
    assert!(grid.iter().any(|c| c.ch != ' '));
}

#[test]
fn playlist_rotates_all_37_effects_without_immediate_repeats() {
    let mut rng = LcgRng::new(42);
    let mut playlist = super::super::effects::EffectPlaylist::new();
    let mut last = None;
    for _ in 0..50 {
        let mut cycle = Vec::with_capacity(37);
        for _ in 0..37 {
            let eff = playlist.next_effect(&mut rng);
            if let Some(prev) = last {
                assert_ne!(eff, prev, "no immediate repeat across cycles");
            }
            last = Some(eff);
            cycle.push(eff);
        }
        cycle.sort_by_key(EffectKind::name);
        cycle.dedup();
        assert_eq!(cycle.len(), 37, "all 37 effects must play in each cycle");
    }
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

    // Resize during active transition must not panic
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
    assert_eq!(saver.last_cols, cols);
    assert_eq!(saver.last_rows, rows);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    for _ in 0..15 {
        saver.update(Duration::from_millis(16), cols, rows);
        saver.draw(&mut grid, cols, rows);
    }
    assert!(grid.iter().any(|c| c.ch != ' '));
}
