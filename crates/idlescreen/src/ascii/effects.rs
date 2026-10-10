// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Effect identity and dispatch for all 37 ttfx visual effects.

use crate::cell::LcgRng;

/// The 37 visual effects supported by ttfx and IdleScreen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
        EffectKind::Beams,
        EffectKind::Binarypath,
        EffectKind::Blackhole,
        EffectKind::Bouncyballs,
        EffectKind::Bubbles,
        EffectKind::Burn,
        EffectKind::Colorshift,
        EffectKind::Crumble,
        EffectKind::Decrypt,
        EffectKind::Errorcorrect,
        EffectKind::Expand,
        EffectKind::Fireworks,
        EffectKind::Highlight,
        EffectKind::Laseretch,
        EffectKind::Matrix,
        EffectKind::Middleout,
        EffectKind::Orbittingvolley,
        EffectKind::Overflow,
        EffectKind::Pour,
        EffectKind::Print,
        EffectKind::Rain,
        EffectKind::Randomsequence,
        EffectKind::Rings,
        EffectKind::Scattered,
        EffectKind::Slice,
        EffectKind::Slide,
        EffectKind::Smoke,
        EffectKind::Spotlights,
        EffectKind::Spray,
        EffectKind::Swarm,
        EffectKind::Sweep,
        EffectKind::Synthgrid,
        EffectKind::Thunderstorm,
        EffectKind::Unstable,
        EffectKind::Vhstape,
        EffectKind::Waves,
        EffectKind::Wipe,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        let key = name.trim().to_ascii_lowercase();
        let stripped = key.replace(['-', '_'], "");
        match stripped.as_str() {
            "beams" => Some(EffectKind::Beams),
            "binarypath" => Some(EffectKind::Binarypath),
            "blackhole" => Some(EffectKind::Blackhole),
            "bouncyballs" => Some(EffectKind::Bouncyballs),
            "bubbles" => Some(EffectKind::Bubbles),
            "burn" => Some(EffectKind::Burn),
            "colorshift" => Some(EffectKind::Colorshift),
            "crumble" => Some(EffectKind::Crumble),
            "decrypt" => Some(EffectKind::Decrypt),
            "errorcorrect" => Some(EffectKind::Errorcorrect),
            "expand" => Some(EffectKind::Expand),
            "fireworks" => Some(EffectKind::Fireworks),
            "highlight" => Some(EffectKind::Highlight),
            "laseretch" => Some(EffectKind::Laseretch),
            "matrix" => Some(EffectKind::Matrix),
            "middleout" => Some(EffectKind::Middleout),
            "orbittingvolley" => Some(EffectKind::Orbittingvolley),
            "overflow" => Some(EffectKind::Overflow),
            "pour" => Some(EffectKind::Pour),
            "print" => Some(EffectKind::Print),
            "rain" | "shower" => Some(EffectKind::Rain),
            "randomsequence" => Some(EffectKind::Randomsequence),
            "rings" => Some(EffectKind::Rings),
            "scattered" | "scramble" => Some(EffectKind::Scattered),
            "slice" => Some(EffectKind::Slice),
            "slide" => Some(EffectKind::Slide),
            "smoke" => Some(EffectKind::Smoke),
            "spotlights" => Some(EffectKind::Spotlights),
            "spray" => Some(EffectKind::Spray),
            "swarm" => Some(EffectKind::Swarm),
            "sweep" | "led" => Some(EffectKind::Sweep),
            "synthgrid" => Some(EffectKind::Synthgrid),
            "thunderstorm" => Some(EffectKind::Thunderstorm),
            "unstable" => Some(EffectKind::Unstable),
            "vhstape" => Some(EffectKind::Vhstape),
            "waves" | "wave" => Some(EffectKind::Waves),
            "wipe" => Some(EffectKind::Wipe),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectKind::Beams => "beams",
            EffectKind::Binarypath => "binarypath",
            EffectKind::Blackhole => "blackhole",
            EffectKind::Bouncyballs => "bouncyballs",
            EffectKind::Bubbles => "bubbles",
            EffectKind::Burn => "burn",
            EffectKind::Colorshift => "colorshift",
            EffectKind::Crumble => "crumble",
            EffectKind::Decrypt => "decrypt",
            EffectKind::Errorcorrect => "errorcorrect",
            EffectKind::Expand => "expand",
            EffectKind::Fireworks => "fireworks",
            EffectKind::Highlight => "highlight",
            EffectKind::Laseretch => "laseretch",
            EffectKind::Matrix => "matrix",
            EffectKind::Middleout => "middleout",
            EffectKind::Orbittingvolley => "orbittingvolley",
            EffectKind::Overflow => "overflow",
            EffectKind::Pour => "pour",
            EffectKind::Print => "print",
            EffectKind::Rain => "rain",
            EffectKind::Randomsequence => "randomsequence",
            EffectKind::Rings => "rings",
            EffectKind::Scattered => "scattered",
            EffectKind::Slice => "slice",
            EffectKind::Slide => "slide",
            EffectKind::Smoke => "smoke",
            EffectKind::Spotlights => "spotlights",
            EffectKind::Spray => "spray",
            EffectKind::Swarm => "swarm",
            EffectKind::Sweep => "sweep",
            EffectKind::Synthgrid => "synthgrid",
            EffectKind::Thunderstorm => "thunderstorm",
            EffectKind::Unstable => "unstable",
            EffectKind::Vhstape => "vhstape",
            EffectKind::Waves => "waves",
            EffectKind::Wipe => "wipe",
        }
    }

    pub fn dwell(&self) -> f32 {
        match self {
            EffectKind::Decrypt | EffectKind::Sweep => 8.0,
            EffectKind::Fireworks => 9.0,
            EffectKind::Thunderstorm => 11.0,
            _ => 10.0,
        }
    }
}

/// Shuffled playlist of all 37 effects ensuring randomized rotation without
/// immediate repeats across cycle boundaries.
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
