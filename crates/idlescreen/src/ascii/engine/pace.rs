// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! The pace of a scene: how its steps are spread over its screen time.
//!
//! A scene's steps are its unit of work: one step moves the scene on by one
//! frame of its own animation. The pace turns the scene's step budget into a
//! rate, so the animation can be stretched over the time the scene is on
//! screen instead of racing through it and standing still afterwards.

use std::time::Duration;

/// The most steps a single display frame may take.
///
/// A frame that fell far behind catches up over the frames that follow it,
/// rather than in one burst.
const MAX_STEPS_PER_FRAME: u64 = 4096;

/// How fast a scene runs.
#[derive(Clone, Copy, Debug)]
pub enum ScenePace {
    /// The effect's arcade speed: a whole number of steps per display frame.
    Arcade(u32),
    /// The scene's steps spread evenly over its slot.
    Slot {
        /// The steps the scene takes to play out.
        budget: u64,
        /// The slot the steps are spread over, in nanoseconds.
        slot_ns: u64,
        /// The real time the scene has been on screen, in nanoseconds.
        elapsed_ns: u64,
        /// The steps the scene has taken so far.
        taken: u64,
        /// Whether the scene has shown its first step yet.
        primed: bool,
    },
}

impl ScenePace {
    /// The arcade pace: `steps_per_frame` steps on every display frame.
    pub fn arcade(steps_per_frame: usize) -> Self {
        Self::Arcade(steps_per_frame.max(1) as u32)
    }

    /// A scene of `budget` steps spread over a slot of `slot_secs`.
    ///
    /// The slot is never shorter than the scene needs at its arcade speed: a
    /// scene is slowed down to fill its slot, never sped up to fit. A scene
    /// that already outlasts the slot keeps the speed it was authored at.
    pub fn slot(budget: u64, arcade_steps: usize, fps: u32, slot_secs: f32) -> Self {
        let fps = u64::from(fps.max(1));
        let arcade = u64::from(arcade_steps.max(1) as u32);
        let natural_ns = budget.saturating_mul(1_000_000_000) / (arcade * fps).max(1);
        let wanted_ns = (slot_secs.max(0.0) * 1_000_000_000.0) as u64;
        Self::Slot {
            budget,
            slot_ns: wanted_ns.max(natural_ns).max(1),
            elapsed_ns: 0,
            taken: 0,
            primed: false,
        }
    }

    /// The steps to take on this display frame.
    ///
    /// The slot pace schedules the steps against the real time that passed,
    /// so the animation keeps its tempo even when the display runs slower
    /// than the configured frame rate. The first step is always taken: the
    /// scene shows its opening frame straight away rather than a blank
    /// screen while the pace catches up.
    pub fn take(&mut self, dt: Duration) -> u32 {
        match self {
            Self::Arcade(steps) => *steps,
            Self::Slot {
                budget,
                slot_ns,
                elapsed_ns,
                taken,
                primed,
            } => {
                if !*primed {
                    *primed = true;
                    return 1;
                }
                let passed = u64::try_from(dt.as_nanos()).unwrap_or(u64::MAX);
                let elapsed = elapsed_ns.saturating_add(passed);
                *elapsed_ns = elapsed;
                // The schedule rounds up: a scene that has measured its length
                // must reach that length inside its slot, not a step short of
                // it, or the last frame would hang on screen.
                let due = (u128::from(elapsed) * u128::from(*budget))
                    .div_ceil(u128::from((*slot_ns).max(1))) as u64;
                let steps = due.saturating_sub(*taken);
                *taken = (*taken).max(due);
                if steps == 0 && *taken >= *budget {
                    // The measured length is spent. Keep the scene asking for
                    // its next frame so it can report that it has ended, and
                    // so a scene that ran longer than measured still plays
                    // itself out.
                    return 1;
                }
                steps.min(MAX_STEPS_PER_FRAME) as u32
            }
        }
    }

    /// The seconds left in the scene's slot.
    pub fn slot_left_secs(&self) -> f32 {
        match self {
            Self::Arcade(_) => 0.0,
            Self::Slot {
                slot_ns,
                elapsed_ns,
                ..
            } => slot_ns.saturating_sub(*elapsed_ns) as f32 / 1_000_000_000.0,
        }
    }
}
