//! laseretch, ported from effects/effect_laseretch.py.
//!
//! The inner LaserEtchIterator.Laser class is the `Laser` struct; its methods
//! live on `LaserEtch` (they need &mut self for event hooks). Sparks are a
//! ParticlePool; reclaim-on-event is an EventAction::Callback dispatched in
//! dispatch_callback (plan.md §4.2).
//!
//! Upstream quirk (effect_laseretch.py:404): `--etch-pattern <group>` parses to
//! a CharacterGroup enum member, but build() tests membership against the
//! enum's NAME strings (`CharacterGroup._member_names_`), which never matches a
//! member — the grouped-etch branch is dead code, pending_chars stays empty,
//! and the effect emits exactly one frame. Verified against the pinned
//! reference (`laseretch --etch-pattern row_top_to_bottom` -> frames=1).
//! Reproduced faithfully; only "algorithm" (the default) etches.
//!
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md): `color_shifted_chars` is created but never
//! used upstream, and the pool's available queue is a deque.

use std::collections::{HashMap, VecDeque};

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
use ttfx::engine::animation::{ExistingColorHandling, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::particles::{ParticlePool, ParticleReset};
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::spanning_tree::RecursiveBacktracker;

mod laseretch_sim;
mod laseretch_step;

/// Callback id: sparks_pool.reclaim(spark, hide=True, deactivate=True).
pub(super) const CB_RECLAIM_SPARK: u32 = 0;
/// --etch-pattern accepts either a CharacterGroup name or the literal
/// "algorithm" (upstream `_etch_pattern_type_parser` dual-type parser).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtchPattern {
    Group(CharacterGroup),
    Algorithm,
}
#[derive(Debug, Clone)]
pub struct LaserEtchConfig {
    pub etch_pattern: EtchPattern,
    pub etch_speed: i64,
    pub etch_delay: i64,
    pub cool_gradient_stops: Vec<Color>,
    pub laser_gradient_stops: Vec<Color>,
    pub spark_gradient_stops: Vec<Color>,
    pub spark_cooling_frames: i64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
}
impl Default for LaserEtchConfig {
    fn default() -> Self {
        Self {
            etch_pattern: EtchPattern::Algorithm,
            etch_speed: 6,
            etch_delay: 0,
            cool_gradient_stops: vec![hex_color("ffe680"), hex_color("ff7b00")],
            laser_gradient_stops: vec![hex_color("ffffff"), hex_color("376cff")],
            spark_gradient_stops: vec![
                hex_color("ffffff"),
                hex_color("ffe680"),
                hex_color("ff7b00"),
                hex_color("1a0900"),
            ],
            spark_cooling_frames: 3,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("ffffff"),
            ],
            final_gradient_steps: vec![8],
            final_gradient_frames: 1,
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
/// LaserEtchIterator.Laser state (methods live on LaserEtch for hooks access).
pub(super) struct Laser {
    pub(super) position: Coord,
    pub(super) beam_chars: Vec<CharId>,
    pub(super) spark_gradient: Gradient,
    pub(super) sparks_pool: ParticlePool,
}
pub struct LaserEtch {
    pub(super) config: LaserEtchConfig,
    pub(super) character_final_color_map: HashMap<CharId, ColorPair>,
    pub(super) pending_chars: Vec<CharId>,
    pub(super) char_delay: i64,
    pub(super) laser: Option<Laser>,
}
/// Laser._make_sparks_pool initialize_sparks closure.
pub(super) fn initialize_spark(
    ctx: &mut EngineCtx,
    spark: CharId,
    spark_colors: &[Color],
    cooling_frames: i64,
) {
    let ch = &mut ctx.terminal.arena[spark.0 as usize];
    ch.layer = 2;
    let input_symbol = ch.input_symbol.clone();
    let uses_pre = ch.uses_input_preexisting_colors;
    let spark_scn = ch.animation.new_scene(false, None, None, "spark", uses_pre);
    if let Some(scene) = ch.animation.scenes.get_mut(&spark_scn) {
        for color in spark_colors {
            let _ = scene.add_frame(
                &input_symbol,
                cooling_frames,
                VisualParams {
                    colors: Some(ColorPair::new(Some(*color), None)),
                    ..Default::default()
                },
            );
        }
    }
}
/// LaserEtchIterator._has_input_colors.
pub(super) fn has_input_colors(ctx: &EngineCtx, id: CharId) -> bool {
    let anim = &ctx.terminal.arena[id.0 as usize].animation;
    anim.input_fg_color.is_some() || anim.input_bg_color.is_some()
}
