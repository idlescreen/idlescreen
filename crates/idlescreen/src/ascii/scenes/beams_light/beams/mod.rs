//! beams, ported from effects/effect_beams.py.

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps, parse_positive_int,
    parse_positive_int_range, parse_symbol,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{Animation, ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod beams_step;
#[allow(unused_imports)]
use beams_step::*;
mod beams_sim;
#[allow(unused_imports)]
use beams_sim::*;

#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct BeamsConfig {
    /// Symbols to use for the beam effect when moving along a row. Strings will be used in sequence to create an animation.
    #[arg(long = "beam-row-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["▂", "▁", "_"])]
    pub beam_row_symbols: Vec<String>,

    /// Symbols to use for the beam effect when moving along a column. Strings will be used in sequence to create an animation.
    #[arg(long = "beam-column-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["▌", "▍", "▎", "▏"])]
    pub beam_column_symbols: Vec<String>,

    /// Number of frames to wait before adding the next group of beams. Beams are added in groups of size random(1, 5).
    #[arg(long = "beam-delay", default_value_t = 3, value_parser = parse_positive_int)]
    pub beam_delay: i64,

    /// Speed range of the beam when moving along a row.
    #[arg(long = "beam-row-speed-range", default_value = "30-90", value_parser = parse_positive_int_range)]
    pub beam_row_speed_range: (i64, i64),

    /// Speed range of the beam when moving along a column.
    #[arg(long = "beam-column-speed-range", default_value = "18-35", value_parser = parse_positive_int_range)]
    pub beam_column_speed_range: (i64, i64),

    /// Space separated, unquoted, list of colors for the beam, a gradient will be created between the colors.
    #[arg(long = "beam-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["ffffff", "00D1FF", "8A008A"])]
    pub beam_gradient_stops: Vec<Color>,

    /// Space separated, unquoted, numbers for the of gradient steps to use.
    #[arg(long = "beam-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["2", "6"])]
    pub beam_gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step. Increase to slow down the gradient animation.
    #[arg(long = "beam-gradient-frames", default_value_t = 2, value_parser = parse_positive_int)]
    pub beam_gradient_frames: i64,

    /// Space separated, unquoted, list of colors for the wipe gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "ffffff"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step. Increase to slow down the gradient animation.
    #[arg(long = "final-gradient-frames", default_value_t = 4, value_parser = parse_positive_int)]
    pub final_gradient_frames: i64,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,

    /// Speed of the final wipe as measured in diagonal groups activated per frame.
    #[arg(long = "final-wipe-speed", default_value_t = 6, value_parser = parse_positive_int)]
    pub final_wipe_speed: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
    Row,
    Column,
}
/// BeamsIterator.Group state (get_next_character lives on Beams for hooks access).
pub(super) struct Group {
    pub(super) characters: Vec<CharId>,
    pub(super) direction: Direction,
    pub(super) speed: f64,
    pub(super) next_character_counter: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Beams,
    FinalWipe,
    Complete,
}
pub struct Beams {
    pub(super) config: BeamsConfig,
    pub(super) pending_groups: Vec<Group>,
    pub(super) active_groups: Vec<Group>,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) final_wipe_groups: Vec<Vec<CharId>>,
    pub(super) delay: i64,
    pub(super) phase: Phase,
}
impl Beams {
    pub fn new(config: BeamsConfig) -> Self {
        Beams {
            config,
            pending_groups: Vec::new(),
            active_groups: Vec::new(),
            character_final_color_map: HashMap::new(),
            final_wipe_groups: Vec::new(),
            delay: 0,
            phase: Phase::Beams,
        }
    }

    /// Group.__init__.
    pub(super) fn make_group(
        &self,
        ctx: &mut EngineCtx,
        mut characters: Vec<CharId>,
        direction: Direction,
    ) -> Group {
        let speed_range = match direction {
            Direction::Row => self.config.beam_row_speed_range,
            Direction::Column => self.config.beam_column_speed_range,
        };
        let speed = ctx.rng.randint(speed_range.0, speed_range.1) as f64 * 0.1;
        match direction {
            Direction::Row => {
                characters.sort_by_key(|&id| ctx.terminal.arena[id.0 as usize].input_coord.column);
            }
            Direction::Column => {
                characters.sort_by_key(|&id| ctx.terminal.arena[id.0 as usize].input_coord.row);
            }
        }
        if *ctx.rng.choice(&[true, false]) {
            characters.reverse();
        }
        Group {
            characters,
            direction,
            speed,
            next_character_counter: 0.0,
        }
    }

    /// Group.get_next_character.
    pub(super) fn get_next_character(
        &mut self,
        ctx: &mut EngineCtx,
        group: &mut Group,
    ) -> Option<CharId> {
        group.next_character_counter -= 1.0;
        let next_character = group.characters.remove(0);
        let active_scene = ctx.terminal.arena[next_character.0 as usize]
            .animation
            .active_scene
            .clone();
        let return_value = if let Some(scene_id) = active_scene {
            ctx.terminal.arena[next_character.0 as usize]
                .animation
                .scenes
                .get_mut(&scene_id)
                .must_val()
                .reset_scene();
            None
        } else {
            ctx.terminal.set_character_visibility(next_character, true);
            Some(next_character)
        };
        let scene_name = match group.direction {
            Direction::Row => "beam_row",
            Direction::Column => "beam_column",
        };
        ctx.activate_scene(self, next_character, scene_name);
        return_value
    }
}
impl EffectHooks for Beams {
    fn dispatch_callback(
        &mut self,
        _ctx: &mut EngineCtx,
        _character: CharId,
        _callback: &EffectCallback,
    ) {
    }
}
