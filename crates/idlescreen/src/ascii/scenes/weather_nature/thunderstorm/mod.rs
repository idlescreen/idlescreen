//! thunderstorm, ported from effects/effect_thunderstorm.py.
//!
//! Two ParticlePools (rain + sparks) plus a manually managed strike-character
//! pool (available/pending/active lists). The storm budget reads the injected
//! monotonic clock at exactly the upstream points (__init__:203, fade_complete
//! :390, __next__:719 — plan.md §4.7). Upstream's three character color maps
//! (character_final/visible/storm_color_map) are written but never read, so
//! they are not stored here. `self.flashing` is likewise write-only upstream.
//! No observable set iteration beyond the engine-canonical active_characters
//! (docs/ordering-inventory.md).

use clap::Parser;

use crate::ascii::scenes::canvas_ops::{
    SceneValueExt, emit_frame, parse_gradient_direction, parse_gradient_steps, parse_positive_int,
    parse_symbol,
};
use ttfx::cli::parse_color;
use ttfx::engine::animation::{Animation, ExistingColorHandling, Scene, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{EffectHooks, EngineCtx, NoopHooks};
use ttfx::engine::effect::Effect;
use ttfx::engine::error::EngineError;
use ttfx::engine::events::{CallerKey, EffectCallback, Event, EventAction};
use ttfx::engine::particles::{ParticlePool, ParticleReset};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use ttfx::utils::pycompat::floor_div;

/// fade_complete (effect_thunderstorm.py:388): phase -> storm, restart clock.
mod thunderstorm_step;
#[allow(unused_imports)]
use thunderstorm_step::*;
mod thunderstorm_sim;
#[allow(unused_imports)]
use thunderstorm_sim::*;
mod thunderstorm_build;
#[allow(unused_imports)]
use thunderstorm_build::*;
mod thunderstorm_fx;
#[allow(unused_imports)]
use thunderstorm_fx::*;

pub(super) const CB_FADE_COMPLETE: u32 = 0;
/// ThunderstormIterator.hide_character.
pub(super) const CB_HIDE_CHARACTER: u32 = 1;
/// ThunderstormIterator.make_char_glow.
pub(super) const CB_MAKE_CHAR_GLOW: u32 = 2;
/// ThunderstormIterator.return_strike_to_pool.
pub(super) const CB_RETURN_STRIKE_TO_POOL: u32 = 3;
/// ThunderstormIterator.set_strike_in_progress_false.
pub(super) const CB_SET_STRIKE_IN_PROGRESS_FALSE: u32 = 4;
/// rain_pool.reclaim_on_event closure (particles.py reclaim).
pub(super) const CB_RECLAIM_RAIN: u32 = 5;
/// spark_pool.reclaim_on_event closure.
pub(super) const CB_RECLAIM_SPARK: u32 = 6;
#[derive(Parser, Debug, Clone)]
#[command(no_binary_name = true)]
pub struct ThunderstormConfig {
    /// Color for the lightning strike.
    #[arg(long = "lightning-color", default_value = "68A3E8", value_parser = parse_color)]
    pub lightning_color: Color,

    /// Color for the text when glowing after a lightning strike.
    #[arg(long = "glowing-text-color", default_value = "EF5411", value_parser = parse_color)]
    pub glowing_text_color: Color,

    /// Number of frames to display each color in the post-lightning text glow cooling gradient. Increase to slow down the cooling animation.
    #[arg(long = "text-glow-time", default_value_t = 3, value_parser = parse_positive_int)]
    pub text_glow_time: i64,

    /// Symbols to use for the raindrops.
    #[arg(long = "raindrop-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["\\", ".", ","])]
    pub raindrop_symbols: Vec<String>,

    /// Symbols to use for the lightning impact sparks.
    #[arg(long = "spark-symbols", num_args = 1.., value_parser = parse_symbol,
          default_values = ["*", ".", "'"])]
    pub spark_symbols: Vec<String>,

    /// Color for the spark glow after a lightning strike.
    #[arg(long = "spark-glow-color", default_value = "ff4d00", value_parser = parse_color)]
    pub spark_glow_color: Color,

    /// Number of frames to display each color in the post-lightning spark cooling gradient. Increase to slow down the cooling animation.
    #[arg(long = "spark-glow-time", default_value_t = 8, value_parser = parse_positive_int)]
    pub spark_glow_time: i64,

    /// Duration, in seconds, the storm will occur.
    #[arg(long = "storm-time", default_value_t = 2, value_parser = parse_positive_int)]
    pub storm_time: i64,

    /// Space separated, unquoted, list of colors for the character gradient (applied across the canvas).
    #[arg(long = "final-gradient-stops", num_args = 1.., value_parser = parse_color,
          default_values = ["8A008A", "00D1FF", "FFFFFF"])]
    pub final_gradient_stops: Vec<Color>,

    /// Number of gradient steps to use.
    #[arg(long = "final-gradient-steps", num_args = 1.., value_parser = parse_gradient_steps,
          default_values = ["12"])]
    pub final_gradient_steps: Vec<i64>,

    /// Number of frames to display each gradient step. Increase to slow down the gradient animation.
    #[arg(long = "final-gradient-frames", default_value_t = 1, value_parser = parse_positive_int)]
    pub final_gradient_frames: i64,

    /// Direction of the final gradient.
    #[arg(long = "final-gradient-direction", default_value = "vertical", value_parser = parse_gradient_direction)]
    pub final_gradient_direction: GradientDirection,
}
/// self.phase string states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    PreStorm,
    Waiting,
    Storm,
    Complete,
}
pub struct Thunderstorm {
    pub(super) config: ThunderstormConfig,
    pub(super) delay: i64,
    pub(super) strike_progression_delay: i64,
    pub(super) rain_pool: ParticlePool,
    pub(super) pending_strike_chars: Vec<CharId>,
    pub(super) available_strike_chars: Vec<CharId>,
    pub(super) active_strike_chars: Vec<CharId>,
    pub(super) spark_pool: ParticlePool,
    /// Gradient captured by the spark initializer closure (build_spark_pool).
    pub(super) spark_gradient: Gradient,
    pub(super) pending_glow_chars: Vec<CharId>,
    pub(super) strike_in_progress: bool,
    pub(super) strike_branch_chance: f64,
    pub(super) phase: Phase,
    pub(super) storm_start_time: f64,
}
/// ThunderstormIterator._adjust_color_pair_brightness.
pub(super) fn adjust_color_pair_brightness(colors: &ColorPair, brightness: f64) -> ColorPair {
    ColorPair::new(
        colors
            .fg_color
            .as_ref()
            .map(|c| Animation::adjust_color_brightness(c, brightness)),
        colors
            .bg_color
            .as_ref()
            .map(|c| Animation::adjust_color_brightness(c, brightness)),
    )
}
/// ThunderstormIterator._add_color_pair_gradient_frames. Faithful quirk: when
/// both endpoint colors exist the gradient list has steps+1 entries but only
/// `range(steps)` of them are emitted as frames.
pub(super) fn add_color_pair_gradient_frames(
    scene: &mut Scene,
    symbol: &str,
    start_colors: &ColorPair,
    end_colors: &ColorPair,
    steps: i64,
    duration: i64,
) -> Result<(), String> {
    let fg_steps: Vec<Option<Color>> =
        if start_colors.fg_color.is_some() && end_colors.fg_color.is_some() {
            Gradient::with_steps(
                &[
                    start_colors.fg_color.clone().must_val(),
                    end_colors.fg_color.clone().must_val(),
                ],
                steps,
                false,
            )?
            .spectrum
            .into_iter()
            .map(Some)
            .collect()
        } else {
            let filler = if end_colors.fg_color.is_some() {
                end_colors.fg_color.clone()
            } else {
                start_colors.fg_color.clone()
            };
            vec![filler; steps as usize]
        };
    let bg_steps: Vec<Option<Color>> =
        if start_colors.bg_color.is_some() && end_colors.bg_color.is_some() {
            Gradient::with_steps(
                &[
                    start_colors.bg_color.clone().must_val(),
                    end_colors.bg_color.clone().must_val(),
                ],
                steps,
                false,
            )?
            .spectrum
            .into_iter()
            .map(Some)
            .collect()
        } else {
            let filler = if end_colors.bg_color.is_some() {
                end_colors.bg_color.clone()
            } else {
                start_colors.bg_color.clone()
            };
            vec![filler; steps as usize]
        };
    for index in 0..steps as usize {
        scene.add_frame(
            symbol,
            duration,
            VisualParams {
                colors: Some(ColorPair::new(
                    fg_steps[index].clone(),
                    bg_steps[index].clone(),
                )),
                ..Default::default()
            },
        )?;
    }
    Ok(())
}
/// build_rain_pool's initialize_raindrop.
pub(super) fn initialize_raindrop(ctx: &mut EngineCtx, id: CharId) {
    let ch = &mut ctx.terminal.arena[id.0 as usize];
    ch.layer = 1;
    let input_symbol = ch.input_symbol.clone();
    let uses_pre = ch.uses_input_preexisting_colors;
    ch.animation.set_appearance(
        &input_symbol,
        uses_pre,
        Some(&input_symbol),
        Some(ColorPair::new(
            Some(Color::from_hex("aaaaff").must_val()),
            None,
        )),
    );
}
