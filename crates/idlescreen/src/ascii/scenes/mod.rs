// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

#![allow(clippy::all, unused)]

macro_rules! declare_scenes {
    ($($name:ident),* $(,)?) => {
        $(
            #[allow(clippy::all, clippy::pedantic, unused)]
            pub mod $name;
        )*
    };
}

declare_scenes!(
    common,
    beams,
    binarypath,
    blackhole,
    bouncyballs,
    bubbles,
    burn,
    colorshift,
    crumble,
    decrypt,
    errorcorrect,
    expand,
    fireworks,
    highlight,
    laseretch,
    matrix,
    middleout,
    orbittingvolley,
    overflow,
    pour,
    print_effect,
    rain,
    random_sequence,
    rings,
    scattered,
    slice,
    slide,
    smoke,
    spotlights,
    spray,
    swarm,
    sweep,
    synthgrid,
    thunderstorm,
    unstable,
    vhstape,
    waves,
    wipe,
);

use super::effects::EffectKind;
use ttfx::engine::effect::Effect;

pub fn create_scene(kind: EffectKind) -> Box<dyn Effect> {
    match kind {
        EffectKind::Beams => Box::new(beams::Beams::default()),
        EffectKind::Binarypath => Box::new(binarypath::BinaryPath::default()),
        EffectKind::Blackhole => Box::new(blackhole::Blackhole::default()),
        EffectKind::Bouncyballs => Box::new(bouncyballs::BouncyBalls::default()),
        EffectKind::Bubbles => Box::new(bubbles::Bubbles::default()),
        EffectKind::Burn => Box::new(burn::Burn::default()),
        EffectKind::Colorshift => Box::new(colorshift::ColorShift::default()),
        EffectKind::Crumble => Box::new(crumble::Crumble::default()),
        EffectKind::Decrypt => Box::new(decrypt::Decrypt::default()),
        EffectKind::Errorcorrect => Box::new(errorcorrect::ErrorCorrect::default()),
        EffectKind::Expand => Box::new(expand::Expand::default()),
        EffectKind::Fireworks => Box::new(fireworks::Fireworks::default()),
        EffectKind::Highlight => Box::new(highlight::Highlight::default()),
        EffectKind::Laseretch => Box::new(laseretch::LaserEtch::default()),
        EffectKind::Matrix => Box::new(matrix::Matrix::default()),
        EffectKind::Middleout => Box::new(middleout::Middleout::default()),
        EffectKind::Orbittingvolley => Box::new(orbittingvolley::OrbittingVolley::default()),
        EffectKind::Overflow => Box::new(overflow::Overflow::default()),
        EffectKind::Pour => Box::new(pour::Pour::default()),
        EffectKind::Print => Box::new(print_effect::Print::default()),
        EffectKind::Rain => Box::new(rain::Rain::default()),
        EffectKind::Randomsequence => Box::new(random_sequence::RandomSequence::default()),
        EffectKind::Rings => Box::new(rings::Rings::default()),
        EffectKind::Scattered => Box::new(scattered::Scattered::default()),
        EffectKind::Slice => Box::new(slice::Slice::default()),
        EffectKind::Slide => Box::new(slide::Slide::default()),
        EffectKind::Smoke => Box::new(smoke::Smoke::default()),
        EffectKind::Spotlights => Box::new(spotlights::Spotlights::default()),
        EffectKind::Spray => Box::new(spray::Spray::default()),
        EffectKind::Swarm => Box::new(swarm::Swarm::default()),
        EffectKind::Sweep => Box::new(sweep::Sweep::default()),
        EffectKind::Synthgrid => Box::new(synthgrid::SynthGrid::default()),
        EffectKind::Thunderstorm => Box::new(thunderstorm::Thunderstorm::default()),
        EffectKind::Unstable => Box::new(unstable::Unstable::default()),
        EffectKind::Vhstape => Box::new(vhstape::VhsTape::default()),
        EffectKind::Waves => Box::new(waves::Waves::default()),
        EffectKind::Wipe => Box::new(wipe::Wipe::default()),
    }
}
