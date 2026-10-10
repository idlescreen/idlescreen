// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Engine session wrapping the genuine ttfx animation engine.

use super::effects::EffectKind;
use clap::Parser;
use ttfx::cli::Cli;
use ttfx::engine::canvas::Anchor;
use ttfx::engine::ctx::{Clock, EngineCtx};
use ttfx::engine::effect::Effect;

pub struct EngineSession {
    ctx: EngineCtx,
    effect: Box<dyn Effect>,
}

impl EngineSession {
    pub fn new(art: &str, kind: EffectKind, cols: usize, rows: usize, seed: u64) -> Option<Self> {
        if cols == 0 || rows == 0 || art.trim().is_empty() {
            return None;
        }
        let cli = Cli::try_parse_from(["ttfx", kind.name()]).ok()?;
        let mut config = cli.terminal_config();
        config.canvas_width = cols as i64;
        config.canvas_height = rows as i64;
        config.ignore_terminal_dimensions = true;
        config.anchor_text = Anchor::C;

        let clock = Clock::virtual_with_frame_rate(60);
        let rng = ttfx::utils::rng::Rng::seeded(seed);
        let mut ctx = EngineCtx::new(art, config, rng, clock).ok()?;
        let mut effect = cli.effect?.build_effect();
        effect.build(&mut ctx).ok()?;

        Some(Self { ctx, effect })
    }

    pub fn next_frame(&mut self) -> Option<String> {
        self.effect.next_frame(&mut self.ctx)
    }
}
