// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! CPU stretch and letterbox upscalers.

mod letterbox;
mod sample;
mod stretch;

// Allocator variants are part of the module API (tests + future callers).
#[allow(unused_imports)]
pub use letterbox::{upscale_letterbox, upscale_letterbox_into};
#[allow(unused_imports)]
pub use stretch::{StretchCache, upscale_stretch, upscale_stretch_into};

#[cfg(test)]
#[path = "../cpu_tests.rs"]
mod tests;
