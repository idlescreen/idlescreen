// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Display power management (DPMS) listener and wake debounce.

use std::time::{Duration, Instant};

pub const DPMS_WAKE_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputPowerMode {
    Off,
    On,
}

pub struct DpmsController {
    is_sleeping: bool,
    wake_at: Option<Instant>,
}

impl Default for DpmsController {
    fn default() -> Self {
        Self::new()
    }
}

impl DpmsController {
    pub fn new() -> Self {
        Self {
            is_sleeping: false,
            wake_at: None,
        }
    }

    pub fn is_sleeping(&self) -> bool {
        self.is_sleeping
    }

    /// Handles power mode transition.
    /// Returns true if wake debounce was initiated.
    pub fn set_power_mode(&mut self, mode: OutputPowerMode) -> bool {
        match mode {
            OutputPowerMode::Off => {
                self.is_sleeping = true;
                self.wake_at = None;
                false
            }
            OutputPowerMode::On => {
                if self.is_sleeping {
                    self.is_sleeping = false;
                    self.wake_at = Some(Instant::now() + DPMS_WAKE_DEBOUNCE);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Returns true if outputs are awake and past the DRM link-training debounce period.
    pub fn is_active_and_stable(&self) -> bool {
        if self.is_sleeping {
            return false;
        }
        if let Some(wake_time) = self.wake_at {
            Instant::now() >= wake_time
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpms_state_transitions() {
        let mut dpms = DpmsController::new();
        assert!(!dpms.is_sleeping());
        assert!(dpms.is_active_and_stable());

        dpms.set_power_mode(OutputPowerMode::Off);
        assert!(dpms.is_sleeping());
        assert!(!dpms.is_active_and_stable());

        let woke = dpms.set_power_mode(OutputPowerMode::On);
        assert!(woke);
        assert!(!dpms.is_sleeping());
    }
}
