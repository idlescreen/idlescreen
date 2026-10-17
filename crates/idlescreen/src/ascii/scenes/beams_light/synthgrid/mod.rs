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

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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

mod synthgrid_build;
mod synthgrid_sim;
mod synthgrid_step;

/// Callback id: update_group_tracker(group_number) — decrements the tracker.
pub(super) const CB_UPDATE_GROUP_TRACKER: u32 = 0;
#[derive(Debug, Clone)]
pub struct SynthGridConfig {
    pub grid_gradient_stops: Vec<Color>,
    pub grid_gradient_steps: Vec<i64>,
    pub grid_gradient_direction: GradientDirection,
    pub text_gradient_stops: Vec<Color>,
    pub text_gradient_steps: Vec<i64>,
    pub text_gradient_direction: GradientDirection,
    pub grid_row_symbol: String,
    pub grid_column_symbol: String,
    pub text_generation_symbols: Vec<String>,
    pub max_active_blocks: f64,
}
impl Default for SynthGridConfig {
    fn default() -> Self {
        Self {
            grid_gradient_stops: vec![hex_color("CC00CC"), hex_color("ffffff")],
            grid_gradient_steps: vec![12],
            grid_gradient_direction: GradientDirection::Diagonal,
            text_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            text_gradient_steps: vec![12],
            text_gradient_direction: GradientDirection::Vertical,
            grid_row_symbol: "─".to_string(),
            grid_column_symbol: "│".to_string(),
            text_generation_symbols: vec!["░".to_string(), "▒".to_string(), "▓".to_string()],
            max_active_blocks: 0.25,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
    Horizontal,
    Vertical,
}
/// GridLine (module-level class in effect_synthgrid.py).
pub(super) struct GridLine {
    pub(super) direction: Direction,
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
