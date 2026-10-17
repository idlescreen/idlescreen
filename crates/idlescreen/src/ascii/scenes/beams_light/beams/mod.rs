//! beams, ported from effects/effect_beams.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{Animation, ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::EffectCallback;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};

mod beams_sim;
mod beams_step;

#[derive(Debug, Clone)]
pub struct BeamsConfig {
    pub beam_row_symbols: Vec<String>,
    pub beam_column_symbols: Vec<String>,
    pub beam_delay: i64,
    pub beam_row_speed_range: (i64, i64),
    pub beam_column_speed_range: (i64, i64),
    pub beam_gradient_stops: Vec<Color>,
    pub beam_gradient_steps: Vec<i64>,
    pub beam_gradient_frames: i64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
    pub final_wipe_speed: i64,
}
impl Default for BeamsConfig {
    fn default() -> Self {
        Self {
            beam_row_symbols: vec!["▂".to_string(), "▁".to_string(), "_".to_string()],
            beam_column_symbols: vec![
                "▌".to_string(),
                "▍".to_string(),
                "▎".to_string(),
                "▏".to_string(),
            ],
            beam_delay: 3,
            beam_row_speed_range: (30, 90),
            beam_column_speed_range: (18, 35),
            beam_gradient_stops: vec![
                hex_color("ffffff"),
                hex_color("00D1FF"),
                hex_color("8A008A"),
            ],
            beam_gradient_steps: vec![2, 6],
            beam_gradient_frames: 2,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("ffffff"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_frames: 4,
            final_gradient_direction: GradientDirection::Vertical,
            final_wipe_speed: 6,
        }
    }
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
                .get_mut(&scene_id)?
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
