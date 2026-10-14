//! synthgrid, ported from effects/effect_synthgrid.py.
//!
//! The module-level GridLine class is the `GridLine` struct; its constructor
//! lives on `Synthgrid` (make_grid_line) for EffectHooks access. Groups are
//! `(group_number, Vec<CharId>)` tuples; the SCENE_COMPLETE-driven group
//! tracker is a Vec indexed by group_number, decremented via an effect
//! callback (upstream EventHandler.Callback(update_group_tracker, n)).
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md).

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_positive_ratio, parse_symbol,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallbackValue, CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, CoordColorMap, Gradient, GradientDirection};
use ttfx::utils::pycompat::floor_div;

mod synthgrid_step;
#[allow(unused_imports)]
use synthgrid_step::*;
mod synthgrid_sim;
#[allow(unused_imports)]
use synthgrid_sim::*;
mod synthgrid_build;
#[allow(unused_imports)]
use synthgrid_build::*;

/// Callback id: update_group_tracker(group_number) — decrements the tracker.
pub(super) const CB_UPDATE_GROUP_TRACKER: u32 = 0;
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct SynthGridConfig {
    /// Space separated, unquoted, list of colors for the grid gradient.
    #[arg(long = "grid-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["CC00CC", "ffffff"])]
    pub grid_gradient_stops: Vec<Color>,

    /// Space separated, unquoted, list of the number of gradient steps to use. More steps will create a smoother and longer gradient animation.
    #[arg(long = "grid-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub grid_gradient_steps: Vec<i64>,

    /// Direction of the gradient for the grid color.
    #[arg(long = "grid-gradient-direction", default_value = "diagonal", value_parser = parse_gradient_direction)]
    pub grid_gradient_direction: GradientDirection,

    /// Space separated, unquoted, list of colors for the text gradient.
    #[arg(long = "text-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "FFFFFF"])]
    pub text_gradient_stops: Vec<Color>,

    /// Space separated, unquoted, list of the number of gradient steps to use. More steps will create a smoother and longer gradient animation.
    #[arg(long = "text-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub text_gradient_steps: Vec<i64>,

    /// Direction of the gradient for the text color.
    #[arg(long = "text-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub text_gradient_direction: GradientDirection,

    /// Symbol to use for grid row lines.
    #[arg(long = "grid-row-symbol", default_value = "─", value_parser = parse_symbol)]
    pub grid_row_symbol: String,

    /// Symbol to use for grid column lines.
    #[arg(long = "grid-column-symbol", default_value = "│", value_parser = parse_symbol)]
    pub grid_column_symbol: String,

    /// Space separated, unquoted, list of characters for the text generation animation.
    #[arg(long = "text-generation-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["░", "▒", "▓"])]
    pub text_generation_symbols: Vec<String>,

    /// Maximum percentage of blocks to have active at any given time. For example, if set to 0.1, 10 percent of the blocks will be active at any given time.
    #[arg(long = "max-active-blocks", default_value_t = 0.25, value_parser = parse_positive_ratio)]
    pub max_active_blocks: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
    Horizontal,
    Vertical,
}
/// GridLine (module-level class in effect_synthgrid.py).
pub(super) struct GridLine {
    pub(super) direction: Direction,
    #[allow(dead_code)]
    pub(super) characters: Vec<CharId>,
    pub(super) collapsed_characters: Vec<CharId>,
    pub(super) extended_characters: Vec<CharId>,
}
/// _phase strings from SynthGridIterator.__next__.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    GridExpand,
    AddChars,
    Collapse,
    Complete,
}
pub struct SynthGrid {
    pub(super) config: SynthGridConfig,
    pub(super) pending_groups: Vec<(i64, Vec<CharId>)>,
    pub(super) grid_lines: Vec<GridLine>,
    /// group_tracker dict, indexed by group_number (keys are 0..n in order).
    pub(super) group_tracker: Vec<i64>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) phase: Phase,
    pub(super) total_group_count: usize,
    pub(super) active_groups: i64,
}
impl GridLine {
    /// GridLine.extend.
    pub(super) fn extend(&mut self, ctx: &mut EngineCtx) {
        let count = if self.direction == Direction::Horizontal {
            3
        } else {
            1
        };
        for _ in 0..count {
            if !self.collapsed_characters.is_empty() {
                let next_char = self.collapsed_characters.remove(0);
                ctx.terminal.set_character_visibility(next_char, true);
                self.extended_characters.push(next_char);
            }
        }
    }

    /// GridLine.collapse.
    pub(super) fn collapse(&mut self, ctx: &mut EngineCtx) {
        let count = if self.direction == Direction::Horizontal {
            3
        } else {
            1
        };
        if self.collapsed_characters.is_empty() {
            self.extended_characters.reverse();
        }
        for _ in 0..count {
            if !self.extended_characters.is_empty() {
                let next_char = self.extended_characters.remove(0);
                ctx.terminal.set_character_visibility(next_char, false);
                self.collapsed_characters.push(next_char);
            }
        }
    }

    /// GridLine.is_extended.
    pub(super) fn is_extended(&self) -> bool {
        self.collapsed_characters.is_empty()
    }

    /// GridLine.is_collapsed.
    pub(super) fn is_collapsed(&self) -> bool {
        self.extended_characters.is_empty()
    }
}
