// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Effect identity and dispatch for all 50 built-in 16-bit arcade visual scenes.

use crate::cell::LcgRng;

/// The 50 visual effects supported by IdleScreen's built-in 16-bit arcade engine.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EffectKind {
    Aurora,
    Beams,
    Binarypath,
    Blackhole,
    Bouncyballs,
    Breakout,
    Bubbles,
    Burn,
    Chaos,
    Colorshift,
    Cosmos,
    Crumble,
    Decrypt,
    Errorcorrect,
    Expand,
    Fireworks,
    Highlight,
    Laseretch,
    Life,
    Matrix,
    Metaballs,
    Middleout,
    Mystify,
    Orbittingvolley,
    Overflow,
    Pipes,
    Plasma,
    Pour,
    Print,
    Radar,
    Rain,
    Randomsequence,
    Rings,
    Ripple,
    Scattered,
    Slice,
    Slide,
    Smoke,
    Spotlights,
    Spray,
    Swarm,
    Sweep,
    Synthgrid,
    Tetris,
    Thunderstorm,
    Tunnel,
    Unstable,
    Vhstape,
    Waves,
    Wipe,
}

impl EffectKind {
    #[rustfmt::skip]
    pub const ALL: [EffectKind; 50] = [
        Self::Aurora, Self::Beams, Self::Binarypath, Self::Blackhole, Self::Bouncyballs,
        Self::Breakout, Self::Bubbles, Self::Burn, Self::Chaos, Self::Colorshift,
        Self::Cosmos, Self::Crumble, Self::Decrypt, Self::Errorcorrect, Self::Expand,
        Self::Fireworks, Self::Highlight, Self::Laseretch, Self::Life, Self::Matrix,
        Self::Metaballs, Self::Middleout, Self::Mystify, Self::Orbittingvolley, Self::Overflow,
        Self::Pipes, Self::Plasma, Self::Pour, Self::Print, Self::Radar,
        Self::Rain, Self::Randomsequence, Self::Rings, Self::Ripple, Self::Scattered,
        Self::Slice, Self::Slide, Self::Smoke, Self::Spotlights, Self::Spray,
        Self::Swarm, Self::Sweep, Self::Synthgrid, Self::Tetris, Self::Thunderstorm,
        Self::Tunnel, Self::Unstable, Self::Vhstape, Self::Waves, Self::Wipe,
    ];

    #[rustfmt::skip]
    pub const NEW_SCENES: [EffectKind; 13] = [
        Self::Aurora, Self::Breakout, Self::Chaos, Self::Cosmos, Self::Life,
        Self::Metaballs, Self::Mystify, Self::Pipes, Self::Plasma, Self::Radar,
        Self::Ripple, Self::Tetris, Self::Tunnel,
    ];

    pub fn step_in_roster(self, roster: &[EffectKind], delta: i32) -> EffectKind {
        if roster.is_empty() {
            return self;
        }
        let cur = roster.iter().position(|&k| k == self).unwrap_or(0) as i32;
        let len = roster.len() as i32;
        let next = (cur + delta).rem_euclid(len) as usize;
        roster[next]
    }

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

    pub fn as_str(&self) -> &'static str {
        self.name()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Aurora => "aurora",
            Self::Beams => "beams",
            Self::Binarypath => "binarypath",
            Self::Blackhole => "blackhole",
            Self::Bouncyballs => "bouncyballs",
            Self::Breakout => "breakout",
            Self::Bubbles => "bubbles",
            Self::Burn => "burn",
            Self::Chaos => "chaos",
            Self::Colorshift => "colorshift",
            Self::Cosmos => "cosmos",
            Self::Crumble => "crumble",
            Self::Decrypt => "decrypt",
            Self::Errorcorrect => "errorcorrect",
            Self::Expand => "expand",
            Self::Fireworks => "fireworks",
            Self::Highlight => "highlight",
            Self::Laseretch => "laseretch",
            Self::Life => "life",
            Self::Matrix => "matrix",
            Self::Metaballs => "metaballs",
            Self::Middleout => "middleout",
            Self::Mystify => "mystify",
            Self::Orbittingvolley => "orbittingvolley",
            Self::Overflow => "overflow",
            Self::Pipes => "pipes",
            Self::Plasma => "plasma",
            Self::Pour => "pour",
            Self::Print => "print",
            Self::Radar => "radar",
            Self::Rain => "rain",
            Self::Randomsequence => "randomsequence",
            Self::Rings => "rings",
            Self::Ripple => "ripple",
            Self::Scattered => "scattered",
            Self::Slice => "slice",
            Self::Slide => "slide",
            Self::Smoke => "smoke",
            Self::Spotlights => "spotlights",
            Self::Spray => "spray",
            Self::Swarm => "swarm",
            Self::Sweep => "sweep",
            Self::Synthgrid => "synthgrid",
            Self::Tetris => "tetris",
            Self::Thunderstorm => "thunderstorm",
            Self::Tunnel => "tunnel",
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

    #[rustfmt::skip]
    pub fn arcade_steps_per_frame(&self) -> usize {
        match self {
            Self::Wipe | Self::Aurora | Self::Breakout | Self::Chaos | Self::Cosmos
            | Self::Life | Self::Metaballs | Self::Mystify | Self::Pipes | Self::Plasma
            | Self::Radar | Self::Ripple | Self::Tetris | Self::Tunnel => 1,
            Self::Slide | Self::Slice | Self::Sweep | Self::Expand
            | Self::Middleout | Self::Highlight | Self::Overflow => 2,
            Self::Spotlights | Self::Matrix | Self::Thunderstorm | Self::Rings
            | Self::Synthgrid | Self::Beams | Self::Fireworks | Self::Rain
            | Self::Orbittingvolley | Self::Crumble | Self::Decrypt => 4,
            Self::Binarypath | Self::Print | Self::Pour | Self::Burn
            | Self::Errorcorrect | Self::Bouncyballs => 5,
            Self::Bubbles | Self::Swarm => 6,
            Self::Blackhole => 7,
            _ => 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EffectPlaylist {
    order: [EffectKind; 50],
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
            cursor: 50,
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
