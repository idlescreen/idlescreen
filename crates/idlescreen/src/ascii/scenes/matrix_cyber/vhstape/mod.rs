//! vhstape, ported from effects/effect_vhstape.py.
//!
//! The inner VHSTapeIterator.Line class is the `Line` struct; lines live in a
//! Vec indexed by row_index (the upstream `lines` dict has keys 0..n-1 in
//! insertion order), and the wave/glitch line lists hold indices (upstream
//! holds Line objects compared by identity). The glitch budget counts frames
//! (`_glitching_steps_elapsed`) — no clock reads. No observable set iteration
//! beyond the engine-canonical active_characters (docs/ordering-inventory.md).

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_ratio, parse_positive_int,
};
use ttfx::cli::parse_color;
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

mod vhstape_step;
#[allow(unused_imports)]
use vhstape_step::*;
mod vhstape_sim;
#[allow(unused_imports)]
use vhstape_sim::*;
mod vhstape_build;
#[allow(unused_imports)]
use vhstape_build::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct VhsTapeConfig {
    /// Space separated, unquoted, list of colors for the characters when a single line is glitching. Colors are applied in order as an animation.
    #[arg(long = "glitch-line-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["ffffff", "ff0000", "00ff00", "0000ff", "ffffff"])]
    pub glitch_line_colors: Vec<Color>,

    /// Space separated, unquoted, list of colors for the characters in lines that are part of the glitch wave. Colors are applied in order as an animation.
    #[arg(long = "glitch-wave-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["ffffff", "ff0000", "00ff00", "0000ff", "ffffff"])]
    pub glitch_wave_colors: Vec<Color>,

    /// Space separated, unquoted, list of colors for the characters during the noise phase.
    #[arg(long = "noise-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["1e1e1f", "3c3b3d", "6d6c70", "a2a1a6", "cbc9cf", "ffffff"])]
    pub noise_colors: Vec<Color>,

    /// Chance that a line will glitch on any given frame.
    #[arg(long = "glitch-line-chance", default_value_t = 0.22, value_parser = parse_non_negative_ratio)]
    pub glitch_line_chance: f64,

    /// Chance that all characters will experience noise on any given frame.
    #[arg(long = "noise-chance", default_value_t = 0.025, value_parser = parse_non_negative_ratio)]
    pub noise_chance: f64,

    /// Total time, frames, that the glitching phase will last.
    #[arg(long = "total-glitch-time", default_value_t = 90, value_parser = parse_positive_int)]
    pub total_glitch_time: i64,

    /// Space separated, unquoted, list of colors for the final color gradient. If only one color is provided, the characters will be displayed in that color.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["ab48ff", "e7b2b2", "fffebd"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
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
