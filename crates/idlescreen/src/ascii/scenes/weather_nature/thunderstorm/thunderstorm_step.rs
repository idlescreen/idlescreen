// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::*;

/// build_spark_pool's _build_spark_characters.
pub(super) fn initialize_spark(
    ctx: &mut EngineCtx,
    id: CharId,
    spark_gradient: &Gradient,
    spark_glow_time: i64,
) {
    let ch = &mut ctx.terminal.arena[id.0 as usize];
    ch.layer = 2;
    let input_symbol = ch.input_symbol.clone();
    let uses_pre = ch.uses_input_preexisting_colors;
    let spark_scn = ch
        .animation
        .new_scene(false, None, Some(Easing::InCirc), "glow", uses_pre);
    let scene = ch.animation.scenes.get_mut(&spark_scn).must_val();
    for color in &spark_gradient.spectrum {
        scene
            .add_frame(
                &input_symbol,
                spark_glow_time,
                VisualParams {
                    colors: Some(ColorPair::new(Some(color.clone()), None)),
                    ..Default::default()
                },
            )
            .must_val();
    }
}
/// ThunderstormIterator._setup_raindrop.
pub(super) fn setup_raindrop(ctx: &mut EngineCtx, id: CharId) {
    let origin = ctx.terminal.arena[id.0 as usize].motion.current_coord;
    let speed = ctx.rng.uniform(0.5, 1.5);
    let canvas_top = ctx.terminal.canvas.top;
    let canvas_bottom = ctx.terminal.canvas.bottom;
    let fall_path = {
        let ch = &mut ctx.terminal.arena[id.0 as usize];
        let path_id = ch
            .motion
            .new_path(speed, None, None, 0, false, "")
            .must_val();
        ch.motion
            .paths
            .get_mut(&path_id)
            .must_val()
            .new_waypoint(
                Coord::new(origin.column + canvas_top + 1, canvas_bottom - 1),
                None,
                "",
            )
            .must_val();
        path_id
    };
    // rain_pool.reclaim_on_event(rain_char, fall_path, event=PATH_COMPLETE)
    ctx.register_event(
        id,
        Event::PathComplete,
        CallerKey::Path(fall_path.clone()),
        EventAction::Callback(EffectCallback {
            id: CB_RECLAIM_RAIN,
            args: Vec::new(),
        }),
    )
    .must_val();
    // PATH_ACTIVATED has no registered handlers on the raindrop (events were
    // cleared this emit; only PATH_COMPLETE is registered), so NoopHooks is
    // observably identical to dispatching through the effect.
    ctx.activate_path(&mut NoopHooks, id, &fall_path);
}
/// ThunderstormIterator._setup_sparks_for_impact.
pub(super) fn setup_sparks_for_impact(ctx: &mut EngineCtx, id: CharId) {
    let impact_coord = ctx.terminal.arena[id.0 as usize].motion.current_coord;
    let speed = ctx.rng.uniform(0.1, 0.25);
    let spark_path = ctx.terminal.arena[id.0 as usize]
        .motion
        .new_path(speed, Some(Easing::OutQuint), None, 30, false, "")
        .must_val();
    let offset = ctx.rng.randint(4, 20) * *ctx.rng.choice(&[1i64, -1]);
    let spark_target = Coord::new(impact_coord.column + offset, ctx.terminal.canvas.bottom);
    let bezier_column =
        impact_coord.column - floor_div(impact_coord.column - spark_target.column, 2);
    let bezier_row = ctx.rng.randint(1, ctx.terminal.canvas.top);
    ctx.terminal.arena[id.0 as usize]
        .motion
        .paths
        .get_mut(&spark_path)
        .must_val()
        .new_waypoint(
            spark_target,
            Some(vec![Coord::new(bezier_column, bezier_row)]),
            "",
        )
        .must_val();
    // spark_pool.reclaim_on_event(spark_char, "glow", event=SCENE_COMPLETE)
    ctx.register_event(
        id,
        Event::SceneComplete,
        CallerKey::Scene("glow".to_string()),
        EventAction::Callback(EffectCallback {
            id: CB_RECLAIM_SPARK,
            args: Vec::new(),
        }),
    )
    .must_val();
    // SCENE_ACTIVATED / PATH_ACTIVATED have no registered handlers here
    // (events were cleared this emit; only SCENE_COMPLETE is registered).
    ctx.activate_scene(&mut NoopHooks, id, "glow");
    ctx.activate_path(&mut NoopHooks, id, &spark_path);
}
impl Thunderstorm {
    pub fn new(config: ThunderstormConfig) -> Self {
        // build_rain_pool / build_spark_pool construction (preallocation runs
        // in build(), where ctx is available, in upstream __init__ order).
        let rain_pool = ParticlePool::new(config.raindrop_symbols.clone(), None, None).must_val();
        let spark_pool =
            ParticlePool::new(config.spark_symbols.clone(), Some(2000), None).must_val();
        Thunderstorm {
            config,
            delay: 0,
            strike_progression_delay: 0,
            rain_pool,
            pending_strike_chars: Vec::new(),
            available_strike_chars: Vec::new(),
            active_strike_chars: Vec::new(),
            spark_pool,
            spark_gradient: Gradient {
                spectrum: Vec::new(),
            },
            pending_glow_chars: Vec::new(),
            strike_in_progress: false,
            strike_branch_chance: 0.05,
            phase: Phase::PreStorm,
            storm_start_time: 0.0,
        }
    }

    /// ThunderstormIterator.build_strike_characters.
    pub(super) fn build_strike_characters(&mut self, ctx: &mut EngineCtx, count: usize) {
        for _ in 0..count {
            let strike_char = ctx.terminal.add_character("|", Coord::new(1, 1));
            self.available_strike_chars.push(strike_char);
        }
    }

    /// ThunderstormIterator.get_next_strike_char.
    pub(super) fn get_next_strike_char(&mut self, ctx: &mut EngineCtx) -> CharId {
        if self.available_strike_chars.is_empty() {
            self.build_strike_characters(ctx, 20);
        }
        let strike_char = self.available_strike_chars.pop().must_val();
        let ch = &mut ctx.terminal.arena[strike_char.0 as usize];
        ch.animation.scenes.clear();
        ch.event_handler.clear();
        strike_char
    }

    /// ThunderstormIterator.setup_lightning_strike (recursive branching).
    pub(super) fn setup_lightning_strike(
        &mut self,
        ctx: &mut EngineCtx,
        branch_neighbor: Option<CharId>,
    ) {
        let mut branch_neighbor = branch_neighbor;
        let (mut column, mut row);
        if let Some(neighbor) = branch_neighbor {
            let coord = ctx.terminal.arena[neighbor.0 as usize].motion.current_coord;
            column = coord.column;
            row = coord.row;
        } else {
            column = ctx.rng.randint(1, ctx.terminal.canvas.right);
            row = ctx.terminal.canvas.top;
        }

        while row >= ctx.terminal.canvas.bottom {
            if self.available_strike_chars.is_empty() {
                self.build_strike_characters(ctx, 20);
            }
            let symbol: &str;
            if let Some(neighbor) = branch_neighbor {
                // Strike characters are all created with input_symbol "|", so
                // the "/" and "\\" arms are unreachable upstream too —
                // transcribed faithfully anyway.
                let neighbor_symbol = ctx.terminal.arena[neighbor.0 as usize].input_symbol.clone();
                if neighbor_symbol == "/" {
                    column += 1;
                    symbol = *ctx.rng.choice(&["|", "\\"]);
                } else if neighbor_symbol == "\\" {
                    column -= 1;
                    symbol = *ctx.rng.choice(&["|", "/"]);
                } else {
                    let delta = *ctx.rng.choice(&[-1i64, 1]);
                    column += delta;
                    symbol = if delta == 1 { "\\" } else { "/" };
                }
            } else {
                symbol = *ctx.rng.choice(&["\\", "/", "|"]);
            }

            let strike_char = self.get_next_strike_char(ctx);
            {
                let ch = &mut ctx.terminal.arena[strike_char.0 as usize];
                ch.motion.set_coordinate(Coord::new(column, row));
                let input_symbol = ch.input_symbol.clone();
                let uses_pre = ch.uses_input_preexisting_colors;
                ch.animation.set_appearance(
                    &input_symbol,
                    uses_pre,
                    Some(symbol),
                    Some(ColorPair::new(
                        Some(self.config.lightning_color.clone()),
                        None,
                    )),
                );
            }
            row -= 1;
            if symbol == "\\" {
                column += 1;
            } else if symbol == "/" {
                column -= 1;
            }

            self.pending_strike_chars.push(strike_char);
            // random.random() is always drawn (left operand of `and`).
            if ctx.rng.random() < self.strike_branch_chance && branch_neighbor.is_none() {
                self.strike_branch_chance -= 0.01;
                self.setup_lightning_strike(ctx, Some(strike_char));
            }
            branch_neighbor = None;
        }
        self.strike_branch_chance = 0.05;
    }
}
