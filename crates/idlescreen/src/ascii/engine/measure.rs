// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Measuring how long a scene takes at the current grid.
//!
//! A scene has no length of its own: it plays until its phases are done, and
//! how long that is depends on the grid, the banner and the seed. The only
//! way to know is to run it. The measurement draws none of it, so it costs
//! the scene's simulation and nothing more.

use super::build_scene;
use crate::ascii::effects::EffectKind;
use crate::ascii::scenes::canvas_ops::set_defer_frame_render;

/// The step count at which a scene is assumed never to end.
///
/// The longest scene measured runs to a little over a thousand steps. This
/// guard is far past anything real, and it keeps a scene that never finishes
/// from hanging the daemon.
const MAX_STEPS: u64 = 20_000;

/// The steps the scene takes to play out at this grid and seed.
///
/// `None` when the scene cannot be built, or when it runs past [`MAX_STEPS`].
pub fn measure_steps(
    art: &str,
    kind: EffectKind,
    cols: usize,
    rows: usize,
    seed: u64,
    fps: u32,
) -> Option<u64> {
    let (mut ctx, mut effect) = build_scene(art, kind, cols, rows, seed, fps)?;
    let mut steps = 0u64;
    set_defer_frame_render(true);
    while effect.next_frame(&mut ctx).is_some() {
        steps += 1;
        if steps >= MAX_STEPS {
            set_defer_frame_render(false);
            return None;
        }
    }
    set_defer_frame_render(false);
    Some(steps)
}
