// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! IdleScreen core library modules.
//!
//! The crate is split into two halves. `ascii`, `cell` and `raster` are the
//! rendering core: the 53 built-in scenes, the terminal cell model, and the
//! pure-CPU rasterizer that turns a cell grid into pixels. They carry no
//! product dependencies, so a headless consumer — the offline renderer in
//! the studio repo — can use them without D-Bus or a Wayland compositor.
//!
//! `cli`, `daemon`, `tui` and `wayland` are the product: they talk to the
//! desktop session and are gated behind the `product` feature.

#![deny(unsafe_code)]

pub mod ascii;
pub mod cell;
pub mod raster;

#[cfg(feature = "product")]
pub mod cli;
#[cfg(feature = "product")]
pub mod daemon;
#[cfg(feature = "product")]
pub mod tui;
#[cfg(feature = "product")]
pub mod wayland;
