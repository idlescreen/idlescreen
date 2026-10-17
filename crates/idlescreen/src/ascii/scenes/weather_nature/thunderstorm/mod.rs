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

use crate::ascii::scenes::canvas_ops::{emit_frame, hex_color};
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
mod thunderstorm_build;
mod thunderstorm_fx;
mod thunderstorm_sim;
mod thunderstorm_step;
use thunderstorm_step::{initialize_spark, setup_raindrop, setup_sparks_for_impact};

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
#[derive(Debug, Clone)]
pub struct ThunderstormConfig {
    pub lightning_color: Color,
    pub glowing_text_color: Color,
    pub text_glow_time: i64,
    pub raindrop_symbols: Vec<String>,
    pub spark_symbols: Vec<String>,
    pub spark_glow_color: Color,
    pub spark_glow_time: i64,
    pub storm_time: i64,
    pub final_gradient_stops: Vec<Color>,
    pub final_gradient_steps: Vec<i64>,
    pub final_gradient_frames: i64,
    pub final_gradient_direction: GradientDirection,
}
impl Default for ThunderstormConfig {
    fn default() -> Self {
        Self {
            lightning_color: hex_color("68A3E8"),
            glowing_text_color: hex_color("EF5411"),
            text_glow_time: 3,
            raindrop_symbols: vec!["\\".to_string(), ".".to_string(), ",".to_string()],
            spark_symbols: vec!["*".to_string(), ".".to_string(), "'".to_string()],
            spark_glow_color: hex_color("ff4d00"),
            spark_glow_time: 8,
            storm_time: 2,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_frames: 1,
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
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
        if let (Some(s), Some(e)) = (&start_colors.fg_color, &end_colors.fg_color) {
            Gradient::with_steps(&[*s, *e], steps, false)?
                .spectrum
                .into_iter()
                .map(Some)
                .collect()
        } else {
            let filler = if end_colors.fg_color.is_some() {
                end_colors.fg_color
            } else {
                start_colors.fg_color
            };
            vec![filler; steps as usize]
        };
    let bg_steps: Vec<Option<Color>> =
        if let (Some(s), Some(e)) = (&start_colors.bg_color, &end_colors.bg_color) {
            Gradient::with_steps(&[*s, *e], steps, false)?
                .spectrum
                .into_iter()
                .map(Some)
                .collect()
        } else {
            let filler = if end_colors.bg_color.is_some() {
                end_colors.bg_color
            } else {
                start_colors.bg_color
            };
            vec![filler; steps as usize]
        };
    for index in 0..steps as usize {
        scene.add_frame(
            symbol,
            duration,
            VisualParams {
                colors: Some(ColorPair::new(fg_steps[index], bg_steps[index])),
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
        Some(ColorPair::new(Some(hex_color("aaaaff")), None)),
    );
}

impl EffectHooks for Thunderstorm {
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        callback: &EffectCallback,
    ) {
        match callback.id {
            CB_FADE_COMPLETE => {
                self.phase = Phase::Storm;
                self.storm_start_time = ctx.clock.now_monotonic();
            }
            CB_HIDE_CHARACTER => ctx.terminal.set_character_visibility(character, false),
            CB_MAKE_CHAR_GLOW => {
                let coord = ctx.terminal.arena[character.0 as usize]
                    .motion
                    .current_coord;
                if let Some(input_char) = ctx.terminal.get_character_by_input_coord(coord)
                    && ctx.terminal.arena[input_char.0 as usize].is_visible
                {
                    ctx.activate_scene(self, input_char, "glow");
                    self.pending_glow_chars.push(input_char);
                }
            }
            CB_RETURN_STRIKE_TO_POOL => self.available_strike_chars.push(character),
            CB_SET_STRIKE_IN_PROGRESS_FALSE => self.strike_in_progress = false,
            CB_RECLAIM_RAIN => self.rain_pool.reclaim(ctx, character, true, true),
            CB_RECLAIM_SPARK => self.spark_pool.reclaim(ctx, character, true, true),
            _ => {}
        }
    }
}
