// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Engine session wrapping the in-tree decomposed ASCII animation scenes.

use super::effects::EffectKind;
use super::scenes;
use ttfx::engine::canvas::Anchor;
use ttfx::engine::ctx::{Clock, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::terminal::TerminalConfig;

pub struct EngineSession {
    ctx: EngineCtx,
    effect: Box<dyn Effect>,
    steps_per_frame: usize,
}

impl EngineSession {
    pub fn new(
        art: &str,
        kind: EffectKind,
        cols: usize,
        rows: usize,
        seed: u64,
        fps: u32,
    ) -> Option<Self> {
        if cols == 0 || rows == 0 || art.trim().is_empty() {
            return None;
        }
        let target_fps = if fps > 0 { fps } else { 60 };
        let config = TerminalConfig {
            canvas_width: cols as i64,
            canvas_height: rows as i64,
            ignore_terminal_dimensions: true,
            anchor_text: Anchor::C,
            frame_rate: target_fps as i64,
            ..Default::default()
        };

        let clock = Clock::virtual_with_frame_rate(target_fps as i64);
        let rng = ttfx::utils::rng::Rng::seeded(seed);
        let mut ctx = EngineCtx::new(art, config, rng, clock).ok()?;
        let mut effect = scenes::create_scene(kind);
        effect.build(&mut ctx).ok()?;
        let steps_per_frame = kind.arcade_steps_per_frame();

        Some(Self {
            ctx,
            effect,
            steps_per_frame,
        })
    }

    pub fn next_frame(&mut self) -> Option<String> {
        let mut last = None;
        let steps = self.steps_per_frame.max(1);
        for _ in 0..steps {
            match self.effect.next_frame(&mut self.ctx) {
                Some(f) => last = Some(f),
                None => break,
            }
        }
        last
    }
}
