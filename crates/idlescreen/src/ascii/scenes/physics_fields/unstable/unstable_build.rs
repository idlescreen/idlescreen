// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Default trait implementations for UnstableConfig and Unstable.

use super::{Easing, GradientDirection, Unstable, UnstableConfig, hex_color};

impl Default for UnstableConfig {
    fn default() -> Self {
        Self {
            unstable_color: hex_color("ff9200"),
            explosion_ease: Easing::OutExpo,
            explosion_speed: 1.75,
            reassembly_ease: Easing::OutExpo,
            reassembly_speed: 1.6,
            final_gradient_stops: vec![
                hex_color("8A008A"),
                hex_color("00D1FF"),
                hex_color("FFFFFF"),
            ],
            final_gradient_steps: vec![12],
            final_gradient_direction: GradientDirection::Vertical,
        }
    }
}
impl Default for Unstable {
    fn default() -> Self {
        Self::new(UnstableConfig::default())
    }
}
