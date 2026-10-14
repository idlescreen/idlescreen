// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Default trait implementations for UnstableConfig and Unstable.

use super::*;

impl Default for UnstableConfig {
    fn default() -> Self {
        clap::Parser::parse_from(std::iter::empty::<&str>())
    }
}
impl Default for Unstable {
    fn default() -> Self {
        Self::new(UnstableConfig::default())
    }
}
