// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Buffer pool helpers for the Wayland presenter.

pub mod swap;

pub use swap::{BufferTarget, reclaim_released, select_buffer};
