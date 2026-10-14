// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Effect identity and dispatch for all 37 ttfx visual effects.

use crate::cell::LcgRng;

/// The 37 visual effects supported by ttfx and IdleScreen.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EffectKind {
    Beams,
    Binarypath,
    Blackhole,
    Bouncyballs,
    Bubbles,
    Burn,
    Colorshift,
    Crumble,
    Decrypt,
    Errorcorrect,
    Expand,
    Fireworks,
    Highlight,
    Laseretch,
    Matrix,
    Middleout,
    Orbittingvolley,
    Overflow,
    Pour,
    Print,
    Rain,
    Randomsequence,
    Rings,
    Scattered,
    Slice,
    Slide,
    Smoke,
    Spotlights,
    Spray,
    Swarm,
    Sweep,
    Synthgrid,
    Thunderstorm,
    Unstable,
    Vhstape,
    Waves,
    Wipe,
}

impl EffectKind {
    pub const ALL: [EffectKind; 37] = [
        Self::Beams,
        Self::Binarypath,
        Self::Blackhole,
        Self::Bouncyballs,
        Self::Bubbles,
        Self::Burn,
        Self::Colorshift,
        Self::Crumble,
        Self::Decrypt,
        Self::Errorcorrect,
        Self::Expand,
        Self::Fireworks,
        Self::Highlight,
        Self::Laseretch,
        Self::Matrix,
        Self::Middleout,
        Self::Orbittingvolley,
        Self::Overflow,
        Self::Pour,
        Self::Print,
        Self::Rain,
        Self::Randomsequence,
        Self::Rings,
        Self::Scattered,
        Self::Slice,
        Self::Slide,
        Self::Smoke,
        Self::Spotlights,
        Self::Spray,
        Self::Swarm,
        Self::Sweep,
        Self::Synthgrid,
        Self::Thunderstorm,
        Self::Unstable,
        Self::Vhstape,
        Self::Waves,
        Self::Wipe,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        let key = name.trim().to_ascii_lowercase().replace(['-', '_'], "");
        match key.as_str() {
            "shower" => Some(Self::Rain),
            "scramble" => Some(Self::Scattered),
            "led" => Some(Self::Sweep),
            "wave" => Some(Self::Waves),
            k => Self::ALL.into_iter().find(|e| e.name() == k),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Beams => "beams",
            Self::Binarypath => "binarypath",
            Self::Blackhole => "blackhole",
            Self::Bouncyballs => "bouncyballs",
            Self::Bubbles => "bubbles",
            Self::Burn => "burn",
            Self::Colorshift => "colorshift",
            Self::Crumble => "crumble",
            Self::Decrypt => "decrypt",
            Self::Errorcorrect => "errorcorrect",
            Self::Expand => "expand",
            Self::Fireworks => "fireworks",
            Self::Highlight => "highlight",
            Self::Laseretch => "laseretch",
            Self::Matrix => "matrix",
            Self::Middleout => "middleout",
            Self::Orbittingvolley => "orbittingvolley",
            Self::Overflow => "overflow",
            Self::Pour => "pour",
            Self::Print => "print",
            Self::Rain => "rain",
            Self::Randomsequence => "randomsequence",
            Self::Rings => "rings",
            Self::Scattered => "scattered",
            Self::Slice => "slice",
            Self::Slide => "slide",
            Self::Smoke => "smoke",
            Self::Spotlights => "spotlights",
            Self::Spray => "spray",
            Self::Swarm => "swarm",
            Self::Sweep => "sweep",
            Self::Synthgrid => "synthgrid",
            Self::Thunderstorm => "thunderstorm",
            Self::Unstable => "unstable",
            Self::Vhstape => "vhstape",
            Self::Waves => "waves",
            Self::Wipe => "wipe",
        }
    }

    pub fn dwell(&self) -> f32 {
        match self {
            Self::Decrypt | Self::Sweep => 1.6,
            Self::Fireworks => 1.8,
            Self::Thunderstorm => 2.0,
            _ => 1.8,
        }
    }

    pub fn arcade_steps_per_frame(&self) -> usize {
        match self {
            Self::Wipe => 1,
            Self::Slide
            | Self::Slice
            | Self::Sweep
            | Self::Expand
            | Self::Middleout
            | Self::Highlight
            | Self::Overflow => 2,
            Self::Spotlights
            | Self::Matrix
            | Self::Thunderstorm
            | Self::Rings
            | Self::Synthgrid
            | Self::Beams
            | Self::Fireworks
            | Self::Rain
            | Self::Orbittingvolley
            | Self::Crumble
            | Self::Decrypt => 4,
            Self::Binarypath
            | Self::Print
            | Self::Pour
            | Self::Burn
            | Self::Errorcorrect
            | Self::Bouncyballs => 5,
            Self::Bubbles | Self::Swarm => 6,
            Self::Blackhole => 7,
            _ => 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EffectPlaylist {
    order: [EffectKind; 37],
    cursor: usize,
    last: Option<EffectKind>,
}

impl Default for EffectPlaylist {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectPlaylist {
    pub fn new() -> Self {
        Self::new_with_last(None)
    }

    pub fn new_with_last(last: Option<EffectKind>) -> Self {
        Self {
            order: EffectKind::ALL,
            cursor: 37,
            last,
        }
    }

    pub fn set_last(&mut self, last: EffectKind) {
        self.last = Some(last);
    }

    pub fn next_effect(&mut self, rng: &mut LcgRng) -> EffectKind {
        if self.cursor >= self.order.len() {
            let last_effect = self.last;
            self.shuffle(rng);
            if let Some(last) = last_effect
                && self.order[0] == last
                && self.order.len() > 1
            {
                let swap_idx = 1 + rng.next_usize(self.order.len() - 1);
                self.order.swap(0, swap_idx);
            }
            self.cursor = 0;
        }
        let effect = self.order[self.cursor];
        self.cursor += 1;
        self.last = Some(effect);
        effect
    }

    fn shuffle(&mut self, rng: &mut LcgRng) {
        for i in (1..self.order.len()).rev() {
            let j = rng.next_usize(i + 1);
            self.order.swap(i, j);
        }
    }
}
