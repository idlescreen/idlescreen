// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Engine session wrapping the in-tree decomposed ASCII animation scenes.
//!
//! A scene is a finite animation: it plays through its phases and then ends.
//! The session decides how fast it plays. The arcade pace runs the effect's
//! own speed — a whole number of steps per display frame. The slot pace
//! measures the scene first and then spreads its steps over the whole slot,
//! so the animation plays out instead of racing through and standing still.

mod measure;
mod pace;

use super::effects::EffectKind;
use super::scenes;
use std::time::Duration;
use ttfx::engine::canvas::Anchor;
use ttfx::engine::ctx::{Clock, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::terminal::TerminalConfig;

pub use measure::measure_steps;
pub use pace::ScenePace;

/// What one call to [`EngineSession::next_frame_into`] produced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SceneStep {
    /// The scene advanced: `out` holds a fresh frame.
    Advanced,
    /// The scene is between steps: the last frame still stands.
    Held,
    /// The scene has played out.
    Finished,
}

/// A live scene, advanced one display frame at a time.
pub struct EngineSession {
    ctx: EngineCtx,
    effect: Box<dyn Effect>,
    pace: ScenePace,
    finished: bool,
}

impl EngineSession {
    /// A session that runs the scene at the effect's arcade speed.
    pub fn new(
        art: &str,
        kind: EffectKind,
        cols: usize,
        rows: usize,
        seed: u64,
        fps: u32,
    ) -> Option<Self> {
        let (ctx, effect) = build_scene(art, kind, cols, rows, seed, fps)?;
        Some(Self {
            ctx,
            effect,
            pace: ScenePace::arcade(kind.arcade_steps_per_frame()),
            finished: false,
        })
    }

    /// A session whose animation is paced to fill `slot_secs` of screen time.
    ///
    /// The scene's step budget is measured first: the number of steps it
    /// takes to play out at this grid, with this seed. The budget sets the
    /// pace, so a short scene is slowed down to fill its slot instead of
    /// finishing early and standing still. A scene that already outlasts the
    /// slot keeps the speed it was authored at — it is never sped up to fit.
    pub fn new_paced(
        art: &str,
        kind: EffectKind,
        cols: usize,
        rows: usize,
        seed: u64,
        fps: u32,
        slot_secs: f32,
    ) -> Option<Self> {
        let budget = measure::measure_steps(art, kind, cols, rows, seed, fps);
        let (ctx, effect) = build_scene(art, kind, cols, rows, seed, fps)?;
        let arcade = kind.arcade_steps_per_frame();
        let pace = match budget {
            Some(steps) => ScenePace::slot(steps, arcade, fps, slot_secs),
            None => ScenePace::arcade(arcade),
        };
        Some(Self {
            ctx,
            effect,
            pace,
            finished: false,
        })
    }

    /// Advance the scene by one display frame.
    ///
    /// `dt` is the real time since the last call. The scene's steps are
    /// scheduled against it, so the animation keeps its tempo even when the
    /// display runs slower than the configured frame rate.
    ///
    /// `out` is written only when the scene actually advanced: a held frame
    /// leaves it untouched, so the caller can keep showing what it has.
    pub fn next_frame_into(&mut self, out: &mut String, dt: Duration) -> SceneStep {
        if self.finished {
            return SceneStep::Finished;
        }
        let steps = self.pace.take(dt);
        if steps == 0 {
            return SceneStep::Held;
        }
        let mut advanced = false;
        for _ in 0..steps {
            scenes::canvas_ops::set_defer_frame_render(true);
            let more = self.effect.next_frame(&mut self.ctx).is_some();
            if !more {
                self.finished = true;
                break;
            }
            advanced = true;
        }
        scenes::canvas_ops::set_defer_frame_render(false);
        if !advanced {
            return SceneStep::Finished;
        }
        out.clear();
        out.push_str(&self.ctx.terminal.get_formatted_output_string());
        SceneStep::Advanced
    }

    /// The seconds left in the scene's slot.
    pub fn slot_left_secs(&self) -> f32 {
        self.pace.slot_left_secs()
    }
}

/// Builds a scene and the context it plays in.
fn build_scene(
    art: &str,
    kind: EffectKind,
    cols: usize,
    rows: usize,
    seed: u64,
    fps: u32,
) -> Option<(EngineCtx, Box<dyn Effect>)> {
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
    Some((ctx, effect))
}
