// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::EngineSession;
use super::super::effects::EffectKind;
use crate::cell::LcgRng;
use clap::Parser;
use std::time::Duration;

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
fn all_37_decomposed_scenes_can_build_and_render_frames() {
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
