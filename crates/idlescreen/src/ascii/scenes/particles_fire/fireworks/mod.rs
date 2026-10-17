//! fireworks, ported from effects/effect_fireworks.py.

use std::collections::HashMap;

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::{self, Coord};
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::{floor_div, round_half_even};

mod fireworks_step;

#[derive(Debug, Clone)]
pub struct FireworksConfig {
    pub explode_anywhere: bool,
    pub firework_colors: Vec<Color>,
    pub firework_symbol: String,
    pub firework_volume: f64,
    pub launch_delay: i64,
    pub explode_distance: f64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_direction: GradientDirection,
}
impl Default for FireworksConfig {
    fn default() -> Self {
        Self {
            explode_anywhere: false,
            firework_colors: vec![
                hex_color("88F7E2"),
                hex_color("44D492"),
                hex_color("F5EB67"),
                hex_color("FFA15C"),
                hex_color("FA233E"),
            ],
            firework_symbol: "o".to_string(),
            firework_volume: 0.08,
            launch_delay: 15,
            explode_distance: 0.2,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Horizontal,
        }
    }
}
pub struct Fireworks {
    pub(super) config: FireworksConfig,
    pub(super) shells: Vec<Vec<CharId>>,
    pub(super) firework_volume: i64,
    pub(super) explode_distance: i64,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) launch_delay: i64,
}
impl Fireworks {
    pub fn new(config: FireworksConfig) -> Self {
        Fireworks {
            config,
            shells: Vec::new(),
            firework_volume: 0,
            explode_distance: 0,
            character_final_color_map: HashMap::new(),
            launch_delay: 0,
        }
    }

    /// FireworksIterator.prepare_waypoints.
    pub(super) fn prepare_waypoints(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError> {
        let mut firework_shell: Vec<CharId> = Vec::new();
        let characters = {
            let filter = CharacterFilter::default();
            ctx.terminal
                .get_characters(&mut ctx.rng, filter, CharacterSort::TopToBottomLeftToRight)
        };
        let canvas_bottom = ctx.terminal.canvas.bottom;
        let canvas_top = ctx.terminal.canvas.top;
        let canvas_right = ctx.terminal.canvas.right;
        // Loop variables assigned at each shell boundary and reused across the
        // following characters, like the Python loop-scoped names.
        let mut origin_x: i64 = 0;
        let mut origin_coord = Coord::new(0, 0);
        let mut explode_waypoint_coords: Vec<Coord> = Vec::new();
        for id in characters {
            if firework_shell.len() as i64 == self.firework_volume || firework_shell.is_empty() {
                origin_x = ctx.rng.randrange(0, canvas_right);
                self.shells.push(std::mem::take(&mut firework_shell));
                let min_row = if self.config.explode_anywhere {
                    canvas_bottom
                } else {
                    ctx.terminal.arena[id.0 as usize].input_coord.row
                };
                let origin_y = ctx.rng.randrange(min_row, canvas_top + 1);
                origin_coord = Coord::new(origin_x, origin_y);
                explode_waypoint_coords =
                    geometry::find_coords_in_circle(origin_coord, self.explode_distance);
            }
            let input_coord = ctx.terminal.arena[id.0 as usize].input_coord;
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .set_coordinate(Coord::new(origin_x, canvas_bottom));
                let apex_path = ch
                    .motion
                    .new_path(0.65, Some(Easing::OutExpo), Some(2), 0, false, "apex_pth")
                    .map_err(EngineError::Other)?;
                ch.motion
                    .paths
                    .get_mut(&apex_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(origin_coord, None, "")
                    .map_err(EngineError::Other)?;
            }
            let apex_wpt_coord = origin_coord;
            let explode_speed = ctx.rng.uniform(0.5, 0.9);
            let explode_path = {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .new_path(explode_speed, Some(Easing::OutCirc), Some(2), 0, false, "")
                    .map_err(EngineError::Other)?
            };
            let explode_wpt_coord = *ctx.rng.choice(&explode_waypoint_coords);
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .paths
                    .get_mut(&explode_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(explode_wpt_coord, None, "")
                    .map_err(EngineError::Other)?;
            }

            let bloom_control_point = geometry::extrapolate_along_ray(
                apex_wpt_coord,
                explode_wpt_coord,
                floor_div(self.explode_distance, 2) as f64,
            );
            let bloom_wpt_coord = Coord::new(
                bloom_control_point.column,
                std::cmp::max(1, bloom_control_point.row - 7),
            );
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                ch.motion
                    .paths
                    .get_mut(&explode_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(bloom_wpt_coord, Some(vec![bloom_control_point]), "")
                    .map_err(EngineError::Other)?;
            }
            {
                let ch = &mut ctx.terminal.arena[id.0 as usize];
                let input_path = ch
                    .motion
                    .new_path(
                        0.9,
                        Some(Easing::InOutQuart),
                        Some(2),
                        0,
                        false,
                        "input_pth",
                    )
                    .map_err(EngineError::Other)?;
                let input_control_point = Coord::new(bloom_wpt_coord.column, 1);
                ch.motion
                    .paths
                    .get_mut(&input_path)
                    .ok_or_else(|| EngineError::Other(String::new()))?
                    .new_waypoint(input_coord, Some(vec![input_control_point]), "")
                    .map_err(EngineError::Other)?;
            }
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("apex_pth".to_string()),
                EventAction::ActivatePath(explode_path.clone()),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path(explode_path),
                EventAction::ActivatePath("input_pth".to_string()),
            )
            .map_err(EngineError::Other)?;
            ctx.register_event(
                id,
                Event::PathComplete,
                CallerKey::Path("input_pth".to_string()),
                EventAction::SetLayer(0),
            )
            .map_err(EngineError::Other)?;

            ctx.activate_path(self, id, "apex_pth");

            firework_shell.push(id);
        }
        if !firework_shell.is_empty() {
            self.shells.push(firework_shell);
        }
        Ok(())
    }
}
