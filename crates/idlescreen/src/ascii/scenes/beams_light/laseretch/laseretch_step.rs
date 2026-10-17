// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::{
    CB_RECLAIM_SPARK, CallerKey, CharId, Color, ColorPair, Coord, Easing, EffectCallback,
    EffectHooks, EngineCtx, EngineError, Event, EventAction, Gradient, HashMap, Laser, LaserEtch,
    LaserEtchConfig, ParticlePool, ParticleReset, VecDeque, VisualParams, initialize_spark,
};

impl LaserEtch {
    pub fn new(config: LaserEtchConfig) -> Self {
        LaserEtch {
            config,
            character_final_color_map: HashMap::new(),
            pending_chars: Vec::new(),
            char_delay: 0,
            laser: None,
        }
    }

    /// Laser.__init__ (+ _make_sparks_pool). The pool is created BEFORE the
    /// beam characters, matching upstream's character_id allocation order.
    pub(super) fn make_laser(&mut self, ctx: &mut EngineCtx) -> Result<Laser, EngineError> {
        let mut laser_gradient: VecDeque<Color> =
            Gradient::new(&self.config.laser_gradient_stops, &[6], true, true)
                .map_err(EngineError::Other)?
                .spectrum
                .into();
        let spark_gradient =
            Gradient::new(&self.config.spark_gradient_stops, &[3, 8], false, false)
                .map_err(EngineError::Other)?;

        // Laser._make_sparks_pool
        let mut sparks_pool = ParticlePool::new(
            vec![".".to_string(), ",".to_string(), "*".to_string()],
            None,
            None,
        )
        .map_err(EngineError::Other)?;
        let spark_colors = spark_gradient.spectrum.clone();
        let cooling_frames = self.config.spark_cooling_frames;
        sparks_pool
            .preallocate(ctx, 2000, |ctx, spark| {
                initialize_spark(ctx, spark, &spark_colors, cooling_frames)
            })
            .map_err(EngineError::Other)?;
        for &spark in &sparks_pool.particles {
            ctx.register_event(
                spark,
                Event::SceneComplete,
                CallerKey::Scene("spark".to_string()),
                EventAction::Callback(EffectCallback {
                    id: CB_RECLAIM_SPARK,
                    args: Vec::new(),
                }),
            )
            .map_err(EngineError::Other)?;
        }

        // beam characters up the diagonal from (0, 0)
        let mut row: i64 = 0;
        let mut col: i64 = 0;
        let mut beam_chars: Vec<CharId> = Vec::new();
        while row <= ctx.terminal.canvas.top {
            let symbol = if beam_chars.is_empty() { "*" } else { "/" };
            let char_id = ctx.terminal.add_character(symbol, Coord::new(col, row));
            ctx.terminal.arena[char_id.0 as usize].layer = 2;
            ctx.terminal.set_character_visibility(char_id, true);
            row += 1;
            col += 1;
            beam_chars.push(char_id);
            {
                let ch = &mut ctx.terminal.arena[char_id.0 as usize];
                let input_symbol = ch.input_symbol.clone();
                let uses_pre = ch.uses_input_preexisting_colors;
                let laser_scn = ch.animation.new_scene(true, None, None, "laser", uses_pre);
                let scene = ch
                    .animation
                    .scenes
                    .get_mut(&laser_scn)
                    .ok_or_else(|| EngineError::Other(String::new()))?;
                for color in &laser_gradient {
                    scene
                        .add_frame(
                            &input_symbol,
                            3,
                            VisualParams {
                                colors: Some(ColorPair::new(Some(*color), None)),
                                ..Default::default()
                            },
                        )
                        .map_err(EngineError::Other)?;
                }
            }
            // deque.rotate(-1)
            if let Some(front) = laser_gradient.pop_front() {
                laser_gradient.push_back(front);
            }
            ctx.activate_scene(self, char_id, "laser");
        }
        Ok(Laser {
            position: Coord::new(0, 0),
            beam_chars,
            spark_gradient,
            sparks_pool,
        })
    }

    /// Laser.reposition.
    pub(super) fn laser_reposition(&mut self, ctx: &mut EngineCtx, target: Coord) {
        let Some(mut laser) = self.laser.take() else {
            return;
        };
        laser.position = target;
        for (idx, &char_id) in laser.beam_chars.iter().enumerate() {
            let offset = idx as i64;
            ctx.terminal.arena[char_id.0 as usize]
                .motion
                .set_coordinate(Coord::new(target.column + offset, target.row + offset));
        }
        self.laser_emit_sparks(ctx, &mut laser, 1);
        self.laser = Some(laser);
    }

    /// Laser.emit_sparks (+ its setup_spark_path closure).
    pub(super) fn laser_emit_sparks(
        &mut self,
        ctx: &mut EngineCtx,
        laser: &mut Laser,
        spark_count: usize,
    ) {
        let spark_colors = laser.spark_gradient.spectrum.clone();
        let cooling_frames = self.config.spark_cooling_frames;
        let position = laser.position;
        let bottom = ctx.terminal.canvas.bottom;
        for _ in 0..spark_count {
            laser.sparks_pool.emit(
                ctx,
                position,
                None,
                true,
                ParticleReset::default(),
                |ctx, spark| initialize_spark(ctx, spark, &spark_colors, cooling_frames),
                |ctx, spark| {
                    // setup_spark_path
                    ctx.terminal.arena[spark.0 as usize]
                        .motion
                        .set_coordinate(position);
                    let Ok(spark_path) = ctx.terminal.arena[spark.0 as usize].motion.new_path(
                        0.3,
                        Some(Easing::OutSine),
                        None,
                        0,
                        false,
                        "",
                    ) else {
                        return;
                    };
                    let fall_target_coord = Coord::new(
                        ctx.rng.randint(position.column - 20, position.column + 20),
                        bottom,
                    );
                    let control = Coord::new(
                        fall_target_coord.column,
                        position.row + ctx.rng.randint(-10, 20),
                    );
                    if let Some(p) = ctx.terminal.arena[spark.0 as usize]
                        .motion
                        .paths
                        .get_mut(&spark_path)
                    {
                        let _ = p.new_waypoint(fall_target_coord, Some(vec![control]), "");
                    }
                    ctx.activate_path(self, spark, &spark_path);
                    ctx.activate_scene(self, spark, "spark");
                },
            );
        }
    }

    /// Laser.disable.
    pub(super) fn laser_disable(&mut self, ctx: &mut EngineCtx) {
        let Some(laser) = self.laser.as_ref() else {
            return;
        };
        let beam_chars = laser.beam_chars.clone();
        for char_id in beam_chars {
            ctx.terminal.set_character_visibility(char_id, false);
        }
    }
}
impl EffectHooks for LaserEtch {
    fn dispatch_callback(
        &mut self,
        ctx: &mut EngineCtx,
        character: CharId,
        callback: &EffectCallback,
    ) {
        if callback.id == CB_RECLAIM_SPARK
            && let Some(laser) = self.laser.as_mut()
        {
            laser.sparks_pool.reclaim(ctx, character, true, true);
        }
    }
}
