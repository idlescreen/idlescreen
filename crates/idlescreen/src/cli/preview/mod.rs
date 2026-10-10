// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal screensaver preview using crossterm for headless and desktop sessions.

pub mod options;
pub mod render;
pub mod term_guard;

pub use options::{PreviewOptions, parse_preview_args};
pub use render::execute;
