//! vhstape, ported from effects/effect_vhstape.py.
//!
//! The inner VHSTapeIterator.Line class is the `Line` struct; lines live in a
//! Vec indexed by row_index (the upstream `lines` dict has keys 0..n-1 in
//! insertion order), and the wave/glitch line lists hold indices (upstream
//! holds Line objects compared by identity). The glitch budget counts frames
//! (`_glitching_steps_elapsed`) — no clock reads. No observable set iteration
//! beyond the engine-canonical active_characters (docs/ordering-inventory.md).

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::round_half_even;

mod vhstape_build;
mod vhstape_sim;
mod vhstape_step;

#[derive(Debug, Clone)]
pub struct VhsTapeConfig {
    pub glitch_line_colors: Vec<Color>,
    pub glitch_wave_colors: Vec<Color>,
    pub noise_colors: Vec<Color>,
    pub glitch_line_chance: f64,
    pub noise_chance: f64,
    pub total_glitch_time: i64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for VhsTapeConfig {
    fn default() -> Self {
        Self {
            glitch_line_colors: vec![
                hex_color("ffffff"),
                hex_color("ff0000"),
                hex_color("00ff00"),
                hex_color("0000ff"),
                hex_color("ffffff"),
            ],
            glitch_wave_colors: vec![
                hex_color("ffffff"),
                hex_color("ff0000"),
                hex_color("00ff00"),
                hex_color("0000ff"),
                hex_color("ffffff"),
            ],
            noise_colors: vec![
                hex_color("1e1e1f"),
                hex_color("3c3b3d"),
                hex_color("6d6c70"),
                hex_color("a2a1a6"),
                hex_color("cbc9cf"),
                hex_color("ffffff"),
            ],
            glitch_line_chance: 0.22,
            noise_chance: 0.025,
            total_glitch_time: 90,
            final_gradient_stops: vec![
                hex_color("ab48ff"),
                hex_color("e7b2b2"),
                hex_color("fffebd"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
/// VHSTapeIterator.Line state (methods live on VhsTape for hooks access).
pub(super) struct Line {
    pub(super) characters: Vec<CharId>,
}
/// __next__ phase strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Glitching,
    Noise,
    Redraw,
    Complete,
}
pub struct VhsTape {
    pub(super) config: VhsTapeConfig,
    pub(super) lines: Vec<Line>,
    /// Upstream `active_glitch_wave_top: int | None`.
    pub(super) active_glitch_wave_top: Option<i64>,
    /// Indices into `lines` (upstream holds Line objects, identity-compared).
    pub(super) active_glitch_wave_lines: Vec<usize>,
    pub(super) active_glitch_lines: Vec<usize>,
    pub(super) character_stable_color_map: HashMap<CharId, ColorPair>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) glitching_steps_elapsed: i64,
    pub(super) phase: Phase,
    pub(super) to_redraw: Vec<usize>,
    pub(super) redrawing: bool,
}
impl VhsTape {
    pub fn new(config: VhsTapeConfig) -> Self {
        VhsTape {
            config,
            lines: Vec::new(),
            active_glitch_wave_top: None,
            active_glitch_wave_lines: Vec::new(),
            active_glitch_lines: Vec::new(),
            character_stable_color_map: HashMap::new(),
            character_final_color_map: HashMap::new(),
            glitching_steps_elapsed: 0,
            phase: Phase::Glitching,
            to_redraw: Vec::new(),
            redrawing: false,
        }
    }
}
