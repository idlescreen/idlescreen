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

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps,
    parse_non_negative_int, parse_positive_int,
};
use ttfx::cli::parse_color;
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

mod laseretch_step;
#[allow(unused_imports)]
use laseretch_step::*;
mod laseretch_sim;
#[allow(unused_imports)]
use laseretch_sim::*;

/// Callback id: sparks_pool.reclaim(spark, hide=True, deactivate=True).
pub(super) const CB_RECLAIM_SPARK: u32 = 0;
/// --etch-pattern accepts either a CharacterGroup name or the literal
/// "algorithm" (upstream `_etch_pattern_type_parser` dual-type parser).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtchPattern {
    Group(CharacterGroup),
    Algorithm,
}
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct LaserEtchConfig {
    /// Pattern used to etch the text.
    #[arg(long = "etch-pattern", default_value = "algorithm", value_parser = parse_etch_pattern)]
    pub etch_pattern: EtchPattern,

    /// Along with etch_delay, determines the speed at which the characters are etched onto the terminal. This value specifies the number of characters to etch simultaneously.
    #[arg(long = "etch-speed", default_value_t = 6, value_parser = parse_positive_int)]
    pub etch_speed: i64,

    /// Along with etch_speed, determines the speed at which the characters are etched onto the terminal. This values specifies the number of frames to wait before etching the next set of characters.
    #[arg(long = "etch-delay", default_value_t = 0, value_parser = parse_non_negative_int)]
    pub etch_delay: i64,

    /// Space separated, unquoted, list of colors for the gradient used to cool the characters after etching.
    #[arg(long = "cool-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["ffe680", "ff7b00"])]
    pub cool_gradient_stops: Vec<Color>,

    /// Space separated, unquoted, list of colors for the laser gradient.
    #[arg(long = "laser-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["ffffff", "376cff"])]
    pub laser_gradient_stops: Vec<Color>,

    /// Space separated, unquoted, list of colors for the spark cooling gradient.
    #[arg(long = "spark-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["ffffff", "ffe680", "ff7b00", "1a0900"])]
    pub spark_gradient_stops: Vec<Color>,

    /// Number of frames to display each spark cooling gradient step. Increase to slow down the rate of cooling.
    #[arg(long = "spark-cooling-frames", default_value_t = 3, value_parser = parse_positive_int)]
    pub spark_cooling_frames: i64,

    /// Space separated, unquoted, list of colors for the character gradient (applied across the canvas).
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "ffffff"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["8"])]
    pub final_gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step. Increase to slow down the gradient animation.
    #[arg(long = "final-gradient-frames", default_value_t = 1, value_parser = parse_positive_int)]
    pub final_gradient_frames: i64,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
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
pub(super) fn parse_etch_pattern(s: &str) -> Result<EtchPattern, String> {
    if s == "algorithm" {
        return Ok(EtchPattern::Algorithm);
    }
    crate::ascii::scenes::canvas_ops::parse_character_group(s).map(EtchPattern::Group)
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
    let scene = ch.animation.scenes.get_mut(&spark_scn).must_val();
    for color in spark_colors {
        scene
            .add_frame(
                &input_symbol,
                cooling_frames,
                VisualParams {
                    colors: Some(ColorPair::new(Some(color.clone()), None)),
                    ..Default::default()
                },
            )
            .must_val();
    }
}
/// LaserEtchIterator._has_input_colors.
pub(super) fn has_input_colors(ctx: &EngineCtx, id: CharId) -> bool {
    let anim = &ctx.terminal.arena[id.0 as usize].animation;
    anim.input_fg_color.is_some() || anim.input_bg_color.is_some()
}
