// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

#![allow(clippy::all, clippy::pedantic, unused)]

pub mod arcade_retro;
pub mod field_ops;
pub mod math_fields;
pub mod rain;
pub mod spray;
pub mod thunderstorm;
pub mod waves;

pub use arcade_retro::{breakout, life, mmx, mystify, pipes, tetris, tunnel, zero};
pub use math_fields::{aurora, chaos, cosmos, metaballs, plasma, radar, ripple};
