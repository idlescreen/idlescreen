// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::EngineSession;
use super::super::effects::EffectKind;
use crate::cell::LcgRng;
use clap::Parser;
use std::time::Duration;

#[rustfmt::skip]
const ALL_37_TTFX_NAMES: &[&str] = &[
    "beams", "binarypath", "blackhole", "bouncyballs", "bubbles", "burn",
    "colorshift", "crumble", "decrypt", "errorcorrect", "expand", "fireworks",
    "highlight", "laseretch", "matrix", "middleout", "orbittingvolley",
    "overflow", "pour", "print", "rain", "randomsequence", "rings",
    "scattered", "slice", "slide", "smoke", "spotlights", "spray",
    "swarm", "sweep", "synthgrid", "thunderstorm", "unstable", "vhstape",
    "waves", "wipe",
];

#[rustfmt::skip]
const ALL_52_NAMES: &[&str] = &[
    "aurora", "beams", "binarypath", "blackhole", "bouncyballs", "breakout",
    "bubbles", "burn", "chaos", "colorshift", "cosmos", "crumble",
    "decrypt", "errorcorrect", "expand", "fireworks", "highlight", "laseretch",
    "life", "matrix", "metaballs", "middleout", "mmx", "mystify",
    "orbittingvolley", "overflow", "pipes", "plasma", "pour", "print",
    "radar", "rain", "randomsequence", "rings", "ripple", "scattered",
    "slice", "slide", "smoke", "spotlights", "spray", "swarm",
    "sweep", "synthgrid", "tetris", "thunderstorm", "tunnel", "unstable",
    "vhstape", "waves", "wipe", "zero",
];

#[test]
fn all_52_effects_can_be_parsed() {
    assert_eq!(ALL_52_NAMES.len(), 52);
    assert_eq!(EffectKind::ALL.len(), 52);
    assert_eq!(EffectKind::NEW_SCENES.len(), 13);
    for name in ALL_52_NAMES {
        let kind = EffectKind::parse(name);
        assert!(kind.is_some(), "EffectKind::parse failed for {name}");
        assert_eq!(kind.unwrap().name(), *name);
    }
    assert_eq!(EffectKind::parse("megaman"), Some(EffectKind::Mmx));
    assert_eq!(EffectKind::parse("megamanx"), Some(EffectKind::Mmx));
    assert_eq!(EffectKind::parse("zsaber"), Some(EffectKind::Zero));
    for name in ALL_37_TTFX_NAMES {
        let cli = ttfx::cli::Cli::try_parse_from(["ttfx", name]).unwrap();
        assert!(cli.effect.is_some(), "failed to parse ttfx effect: {name}");
    }
}

#[test]
fn playlist_rotates_all_52_effects_without_immediate_repeats() {
    let mut rng = LcgRng::new(42);
    let mut playlist = super::super::effects::EffectPlaylist::new();
    let mut last = None;
    for _ in 0..30 {
        let mut cycle = Vec::with_capacity(52);
        for _ in 0..52 {
            let eff = playlist.next_effect(&mut rng);
            if let Some(prev) = last {
                assert_ne!(eff, prev, "no immediate repeat across cycles");
            }
            last = Some(eff);
            cycle.push(eff);
        }
        cycle.sort_by_key(EffectKind::name);
        cycle.dedup();
        assert_eq!(cycle.len(), 52, "all 52 effects must play in each cycle");
    }
}

#[test]
fn all_52_decomposed_scenes_can_build_and_render_frames() {
    let art_data = "IDLESCREEN\nSCREENSAVER";
    for kind in EffectKind::ALL {
        let mut effect = crate::ascii::scenes::create_scene(kind);
        let config = ttfx::engine::terminal::TerminalConfig {
            canvas_width: 80,
            canvas_height: 24,
            ignore_terminal_dimensions: true,
            anchor_text: ttfx::engine::canvas::Anchor::C,
            frame_rate: 60,
            ..Default::default()
        };
        let clock = ttfx::engine::ctx::Clock::virtual_with_frame_rate(60);
        let rng = ttfx::utils::rng::Rng::seeded(42);
        let mut ctx = ttfx::engine::ctx::EngineCtx::new(art_data, config, rng, clock).unwrap();
        effect.build(&mut ctx).unwrap();
        let mut frame_count = 0;
        while effect.next_frame(&mut ctx).is_some() {
            frame_count += 1;
            assert!(
                frame_count <= 5000,
                "Decomposed scene {} exceeded 5000 frames",
                kind.name()
            );
        }
        assert!(
            frame_count > 0,
            "Decomposed scene {} must produce at least one frame",
            kind.name()
        );
    }
}

#[test]
fn step_in_roster_wraps_forward_and_backward() {
    for roster in [&EffectKind::ALL[..], &EffectKind::NEW_SCENES[..]] {
        let first = roster[0];
        let last = roster[roster.len() - 1];
        assert_eq!(first.step_in_roster(roster, -1), last);
        assert_eq!(last.step_in_roster(roster, 1), first);
        assert_eq!(first.step_in_roster(roster, 1), roster[1]);
    }
}

#[test]
fn simulation_clock_synchronization_60fps_vs_144fps() {
    let art = "IDLESCREEN";
    let session_60 = EngineSession::new(art, EffectKind::Wipe, 80, 24, 42, 60);
    assert!(session_60.is_some());

    let session_144 = EngineSession::new(art, EffectKind::Wipe, 80, 24, 42, 144);
    assert!(session_144.is_some());

    let session_auto = EngineSession::new(art, EffectKind::Wipe, 80, 24, 42, 0);
    assert!(session_auto.is_some());
}

#[test]
fn wayland_refresh_rate_to_fps_and_duration_calculation() {
    let mhz_60 = 60_000;
    let fps_60 = ((mhz_60 + 500) / 1000).max(1) as u32;
    assert_eq!(fps_60, 60);
    let dur_60 = Duration::from_nanos(1_000_000_000 / fps_60 as u64);
    assert_eq!(dur_60.as_millis(), 16);

    let mhz_144 = 144_000;
    let fps_144 = ((mhz_144 + 500) / 1000).max(1) as u32;
    assert_eq!(fps_144, 144);
    let dur_144 = Duration::from_nanos(1_000_000_000 / fps_144 as u64);
    assert_eq!(dur_144.as_millis(), 6);

    let mhz_5994 = 59_940;
    let fps_5994 = ((mhz_5994 + 500) / 1000).max(1) as u32;
    assert_eq!(fps_5994, 60);
}

#[test]
fn zero_alloc_steady_state() {
    let (cols, rows) = (80, 24);
    let mut saver = super::super::Ascii::new_superfedora().with_dwell(10.0);
    saver.init(cols, rows);
    let mut grid = vec![crate::cell::TerminalCell::default(); cols * rows];

    for &kind in &EffectKind::ALL {
        saver.pin_effect(kind);
        while !saver.is_settled() {
            saver.update(Duration::from_millis(16), cols, rows);
        }
        let grid_ptr = grid.as_ptr();
        let grid_cap = grid.capacity();
        let frame_grid_ptr = saver.frame_grid.as_ptr();
        let frame_grid_cap = saver.frame_grid.capacity();
        let cur_frame_ptr = saver.current_frame.as_ptr();
        let cur_frame_cap = saver.current_frame.capacity();

        for _ in 0..8 {
            saver.update(Duration::from_millis(16), cols, rows);
            saver.draw(&mut grid, cols, rows);
            crate::wayland::glyph_font::draw_preview_hud(&mut grid, cols, rows, kind, false);
        }

        assert_eq!(grid.as_ptr(), grid_ptr);
        assert_eq!(grid.capacity(), grid_cap);
        assert_eq!(saver.frame_grid.as_ptr(), frame_grid_ptr);
        assert_eq!(saver.frame_grid.capacity(), frame_grid_cap);
        assert_eq!(saver.current_frame.as_ptr(), cur_frame_ptr);
        assert_eq!(saver.current_frame.capacity(), cur_frame_cap);
    }
}

#[test]
fn tour_mode_rotates_through_all_52_scenes_on_dwell() {
    let (cols, rows) = (80, 24);
    let mut saver = super::super::Ascii::new().with_dwell(0.5);
    saver.enable_tour_mode();
    saver.init(cols, rows);
    assert!(saver.tour_mode);
    assert!(!saver.pinned);
    assert_eq!(saver.effect, EffectKind::ALL[0]);

    for &expected in &EffectKind::ALL {
        assert_eq!(saver.effect, expected);
        saver.cycle_effect();
    }
    assert_eq!(saver.effect, EffectKind::ALL[0]);

    saver.step_effect(&EffectKind::ALL, -1);
    assert_eq!(saver.effect, EffectKind::ALL[51]);
    saver.step_effect(&EffectKind::ALL, 1);
    assert_eq!(saver.effect, EffectKind::ALL[0]);
}

#[test]
fn zero_snes_combo_destroys_superfedora_on_floor_and_teleports_out() {
    let a = crate::ascii::scenes::weather_nature::arcade_retro::zero::zero_data::atlas();
    assert!(a.saber_swing1.width >= 60 && a.saber_swing2.width >= 75);
    assert!(a.idle1.width != a.idle2.width && a.wave1.pixels != a.wave2.pixels);
    let (cols, rows) = (120, 36);
    let mut saver = super::super::Ascii::new_superfedora().with_dwell(0.5);
    saver.init(cols, rows);
    saver.pin_effect(EffectKind::Zero);
    let mut grid = vec![crate::cell::TerminalCell::default(); cols * rows];
    let (mut saw_crimson, mut saw_hair, mut saw_saber, mut saw_pillar) =
        (false, false, false, false);
    let mut prev_blocks = usize::MAX;
    for step_idx in 1..=120 {
        saver.update(Duration::from_millis(16), cols, rows);
        let cur_blocks = saver.frame_grid.iter().filter(|c| c.ch == '█').count();
        assert!(cur_blocks <= prev_blocks, "regressed at {step_idx}");
        prev_blocks = cur_blocks;
        if step_idx == 20 {
            let max_row = |pred: fn(char) -> bool| {
                saver
                    .frame_grid
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| pred(c.ch))
                    .map(|(i, _)| i / cols)
                    .max()
                    .unwrap()
            };
            assert_eq!(max_row(|c| matches!(c, '▀' | '▄')), max_row(|c| c == '█'));
        }
        saver.draw(&mut grid, cols, rows);
        for c in &grid {
            saw_crimson |= c.fg == (216, 32, 40) || c.bg == (216, 32, 40);
            saw_hair |= c.fg == (248, 200, 32) || c.bg == (248, 200, 32);
            saw_saber |= c.fg == (64, 255, 120) || c.bg == (64, 255, 120);
            saw_pillar |= c.fg == (232, 104, 255) || c.bg == (232, 104, 255);
        }
    }
    assert!(saw_crimson && saw_hair && saw_saber && saw_pillar && saver.is_settled());
    for &(c, r) in &[(80, 24), (44, 16), (40, 12)] {
        saver.init(c, r);
        saver.pin_effect(EffectKind::Zero);
        for _ in 0..120 {
            saver.update(Duration::from_millis(16), c, r);
        }
        assert!(saver.is_settled());
    }
}
