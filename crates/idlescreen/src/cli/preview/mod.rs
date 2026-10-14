// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Interactive terminal and fullscreen Wayland preview subcommand.

pub mod ansi_out;
pub mod options;
pub mod render;
pub mod term_guard;
#[cfg(test)]
mod tests;

pub use options::{PreviewOptions, parse_preview_args};
pub use render::execute;
