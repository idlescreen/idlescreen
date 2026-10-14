//! burn, ported from effects/effect_burn.py.
//!
//! RNG order mirrors BurnIterator.__init__: PrimsSimple construction (random
//! starting coord) first, then the smoke ParticlePool preallocation (2000
//! symbol choice() draws), then the build() body (algo run to completion),
//! then per-frame randint(2, 4) + smoke emission draws.
//!
//! No new observable set iterations: char_link_order and the pool's available
//! deque are ordered upstream lists; BreadthFirst is not used here
//! (docs/ordering-inventory.md unchanged).

use std::collections::HashMap;

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_ratio,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallbackValue, CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::particles::{ParticlePool, ParticleReset};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::spanning_tree::PrimsSimple;

/// Callback id: EventHandler.Callback(lambda c: self._emit_smoke(c.input_coord, ...)).
mod burn_step;
#[allow(unused_imports)]
use burn_step::*;

pub(super) const CB_EMIT_SMOKE: u32 = 0;
/// Callback id: ParticlePool.reclaim_on_event's reclaim closure.
pub(super) const CB_RECLAIM_SMOKE: u32 = 1;
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct BurnConfig {
    /// Color of the characters before they start to burn.
    #[arg(long = "starting-color", default_value = "837373", value_parser = parse_color)]
    pub starting_color: Color,

    /// Colors transitioned through as the characters burn.
    #[arg(long = "burn-colors", num_args = 1.., value_parser = parse_color,
          default_values = ["ffffff", "fff75d", "fe650d", "8A003C", "510100"])]
    pub burn_colors: Vec<Color>,

    /// Chance a given character will produce smoke while burning. Use 0 for no smoke.
    #[arg(long = "smoke-chance", default_value_t = 0.7, value_parser = parse_non_negative_ratio)]
    pub smoke_chance: f64,

    /// Space separated, unquoted, list of colors for the final color gradient.
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["00c3ff", "ffff1c"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
pub(super) const BURN_CHAR_ORDER: [&str; 9] = ["'", ".", "▖", "▙", "█", "▜", "▀", "▝", "."];
pub(super) const SMOKE_SYMBOLS: [&str; 6] = [".", ",", "'", "`", "#", "*"];
pub struct Burn {
    pub(super) config: BurnConfig,
    pub(super) character_final_color_map: HashMap<CharId, Color>,
    /// PrimsSimple.char_link_order, consumed FIFO in next_frame.
    pub(super) char_link_order: Vec<CharId>,
    /// Option so _emit_smoke can move the pool out of self while the on_emit
    /// closure needs &mut self for event dispatch (see emit_smoke).
    pub(super) smoke_particles: Option<ParticlePool>,
    /// Makes each reclaim Callback registration unique, mirroring Python's
    /// fresh closure object per reclaim_on_event call (identity inequality
    /// lets registrations accumulate upstream instead of raising
    /// DuplicateEventRegistrationError; firing reclaim repeatedly is
    /// idempotent, so behavior is identical).
    pub(super) emission_counter: i64,
}
impl Burn {
    pub fn new(config: BurnConfig) -> Self {
        Burn {
            config,
            character_final_color_map: HashMap::new(),
            char_link_order: Vec::new(),
            smoke_particles: None,
            emission_counter: 0,
        }
    }

    /// BurnIterator._has_input_colors.
    pub(super) fn has_input_colors(ctx: &EngineCtx, id: CharId) -> bool {
        let anim = &ctx.terminal.arena[id.0 as usize].animation;
        anim.input_fg_color.is_some() || anim.input_bg_color.is_some()
    }

    /// BurnIterator._is_burnable.
    pub(super) fn is_burnable(&self, ctx: &EngineCtx, id: CharId) -> bool {
        ctx.terminal.arena[id.0 as usize].input_symbol != " "
            || (ctx.terminal.config.existing_color_handling != ExistingColorHandling::Ignore
                && Self::has_input_colors(ctx, id))
    }

    /// BurnIterator._make_smoke_pool's initialize_smoke: one reusable "smoke"
    /// scene (10-frame 504F4F->C7C7C7 fade) and layer 2. Passed to every pool
    /// call; runs only for newly created particles.
    pub(super) fn initialize_smoke(ctx: &mut EngineCtx, id: CharId) {
        let (input_symbol, uses_pre) = {
            let ch = &ctx.terminal.arena[id.0 as usize];
            (ch.input_symbol.clone(), ch.uses_input_preexisting_colors)
        };
        let gradient = Gradient::with_steps(
            &[
                Color::from_hex("504F4F").must_val(),
                Color::from_hex("C7C7C7").must_val(),
            ],
            9,
            false,
        )
        .must_val();
        let ch = &mut ctx.terminal.arena[id.0 as usize];
        let smoke_scn = ch.animation.new_scene(false, None, None, "smoke", uses_pre);
        let scene = ch.animation.scenes.get_mut(&smoke_scn).must_val();
        for color in &gradient.spectrum {
            scene
                .add_frame(
                    &input_symbol,
                    10,
                    VisualParams {
                        colors: Some(ColorPair::new(Some(color.clone()), None)),
                        ..Default::default()
                    },
                )
                .must_val();
        }
        ch.layer = 2;
    }

    /// BurnIterator._emit_smoke.
    pub(super) fn emit_smoke(&mut self, ctx: &mut EngineCtx, origin: Coord) {
        if ctx.rng.random() > self.config.smoke_chance {
            return;
        }
        // ParticleReset(clear_paths=True, deactivate_path=True,
        // deactivate_scene=True) == the upstream defaults.
        let reset = ParticleReset::default();
        self.emission_counter += 1;
        let emission_id = self.emission_counter;
        // Move the pool out of self so the on_emit closure can borrow self as
        // EffectHooks. Safe: on_emit only fires PATH_ACTIVATED/SCENE_ACTIVATED,
        // for which smoke particles never register actions, so CB_RECLAIM_SMOKE
        // cannot dispatch while the pool is out.
        let mut pool = self.smoke_particles.take().must_val();
        pool.emit(
            ctx,
            origin,
            None,
            true,
            reset,
            Self::initialize_smoke,
            |ctx, next_particle| {
                // on_emit_smoke
                ctx.terminal.arena[next_particle.0 as usize]
                    .animation
                    .scenes
                    .get_mut("smoke")
                    .must_val()
                    .reset_scene();
                let smoke_path = ctx.terminal.arena[next_particle.0 as usize]
                    .motion
                    .new_path(0.85, None, None, 0, false, "")
                    .must_val();
                let rise_target_coord = Coord::new(
                    ctx.rng.randint(origin.column - 4, origin.column + 4),
                    ctx.terminal.canvas.top + 1,
                );
                ctx.terminal.arena[next_particle.0 as usize]
                    .motion
                    .paths
                    .get_mut(&smoke_path)
                    .must_val()
                    .new_waypoint(rise_target_coord, None, "")
                    .must_val();
                ctx.activate_path(self, next_particle, &smoke_path);
                ctx.activate_scene(self, next_particle, "smoke");
                // ParticlePool.reclaim_on_event(next_particle, caller="smoke")
                ctx.register_event(
                    next_particle,
                    Event::SceneComplete,
                    CallerKey::Scene("smoke".to_string()),
                    EventAction::Callback(EffectCallback {
                        id: CB_RECLAIM_SMOKE,
                        args: vec![CallbackValue::Int(emission_id)],
                    }),
                )
                .must_val();
            },
        );
        self.smoke_particles = Some(pool);
    }
}
impl EffectHooks for Burn {
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        callback: &EffectCallback,
    ) {
        match callback.id {
            CB_EMIT_SMOKE => {
                let origin = ctx.terminal.arena[character.0 as usize].input_coord;
                self.emit_smoke(ctx, origin);
            }
            CB_RECLAIM_SMOKE => {
                // reclaim(completed_character, hide=True, deactivate=True)
                self.smoke_particles
                    .as_mut()
                    .must_val()
                    .reclaim(ctx, character, true, true);
            }
            _ => {}
        }
    }
}
