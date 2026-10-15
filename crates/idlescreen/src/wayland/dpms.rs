// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Display power management (`zwlr-output-power-management-v1`) controller and wake debounce.

use std::time::{Duration, Instant};

use wayland_client::QueueHandle;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols_wlr::output_power_management::v1::client::{
    zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1,
    zwlr_output_power_v1::{self, ZwlrOutputPowerV1},
};

pub const DPMS_WAKE_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputPowerMode {
    Off,
    On,
}

pub struct DpmsController {
    is_sleeping: bool,
    wake_at: Option<Instant>,
    timeout_secs: u64,
    power_manager: Option<ZwlrOutputPowerManagerV1>,
    output_powers: Vec<ZwlrOutputPowerV1>,
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
            timeout_secs: 0,
            power_manager: None,
            output_powers: Vec::new(),
        }
    }

    pub fn is_sleeping(&self) -> bool {
        self.is_sleeping
    }

    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    pub fn set_timeout_secs(&mut self, secs: u64) {
        self.timeout_secs = secs;
    }

    pub fn has_power_manager(&self) -> bool {
        self.power_manager.is_some()
    }

    pub fn set_manager(&mut self, manager: ZwlrOutputPowerManagerV1) {
        self.power_manager = Some(manager);
    }

    pub fn bind_outputs<D>(&mut self, outputs: &[WlOutput], qh: &QueueHandle<D>)
    where
        D: wayland_client::Dispatch<ZwlrOutputPowerV1, ()> + 'static,
    {
        let Some(ref mgr) = self.power_manager else {
            return;
        };
        if !self.output_powers.is_empty() {
            return;
        }
        for output in outputs {
            self.output_powers
                .push(mgr.get_output_power(output, qh, ()));
        }
    }

    /// Handles power mode transition and dispatches `zwlr_output_power_v1::set_mode` to bound outputs.
    /// Returns true if wake debounce was initiated.
    pub fn set_power_mode(&mut self, mode: OutputPowerMode) -> bool {
        let wl_mode = match mode {
            OutputPowerMode::Off => zwlr_output_power_v1::Mode::Off,
            OutputPowerMode::On => zwlr_output_power_v1::Mode::On,
        };
        for power in &self.output_powers {
            power.set_mode(wl_mode);
        }
        self.apply_mode_state(mode)
    }

    /// Updates internal sleep/debounce state from compositor `mode` event without echoing `set_mode`.
    pub fn on_compositor_mode(&mut self, mode: OutputPowerMode) -> bool {
        self.apply_mode_state(mode)
    }

    /// Handles compositor `failed` event on an output power control object.
    pub fn on_output_power_failed(&mut self, failed_proxy: &ZwlrOutputPowerV1) {
        if let Some(pos) = self.output_powers.iter().position(|p| p == failed_proxy) {
            let removed = self.output_powers.swap_remove(pos);
            removed.destroy();
        }
        if self.output_powers.is_empty() {
            self.is_sleeping = false;
            self.wake_at = None;
        }
    }

    /// Checks whether `timeout_secs` has elapsed during active presentation and transitions outputs to Off,
    /// or wakes outputs if DPMS was disabled/extended live while sleeping.
    pub fn check_idle_sleep(&mut self, presentation_elapsed: Duration) -> bool {
        if self.is_sleeping {
            if self.timeout_secs == 0
                || presentation_elapsed < Duration::from_secs(self.timeout_secs)
            {
                self.set_power_mode(OutputPowerMode::On);
                return true;
            }
            return false;
        }
        if self.timeout_secs == 0 || self.output_powers.is_empty() || !self.is_active_and_stable() {
            return false;
        }
        if presentation_elapsed >= Duration::from_secs(self.timeout_secs) {
            self.set_power_mode(OutputPowerMode::Off);
            true
        } else {
            false
        }
    }

    /// Restores `OutputPowerMode::On` immediately if currently sleeping.
    pub fn wake_outputs(&mut self) -> bool {
        if self.is_sleeping {
            self.set_power_mode(OutputPowerMode::On)
        } else {
            false
        }
    }

    /// Restores `OutputPowerMode::On` on all outputs and destroys Wayland protocol objects.
    pub fn destroy(&mut self) {
        if self.is_sleeping {
            for power in &self.output_powers {
                power.set_mode(zwlr_output_power_v1::Mode::On);
            }
            self.is_sleeping = false;
            self.wake_at = None;
        }
        for power in self.output_powers.drain(..) {
            power.destroy();
        }
        if let Some(mgr) = self.power_manager.take() {
            mgr.destroy();
        }
    }

    fn apply_mode_state(&mut self, mode: OutputPowerMode) -> bool {
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
        self.wake_at
            .is_none_or(|wake_time| Instant::now() >= wake_time)
    }
}

impl Drop for DpmsController {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpms_state_transitions_and_wake() {
        let mut dpms = DpmsController::new();
        assert!(!dpms.is_sleeping());
        assert_eq!(dpms.timeout_secs(), 0);
        assert!(dpms.is_active_and_stable());

        dpms.set_timeout_secs(300);
        assert_eq!(dpms.timeout_secs(), 300);
        // Without bound Wayland output_powers, check_idle_sleep is a safe no-op
        assert!(!dpms.check_idle_sleep(Duration::from_mins(10)));

        dpms.set_power_mode(OutputPowerMode::Off);
        assert!(dpms.is_sleeping());
        assert!(!dpms.is_active_and_stable());

        let woke = dpms.wake_outputs();
        assert!(woke);
        assert!(!dpms.is_sleeping());
        assert!(!dpms.wake_outputs());
    }

    #[test]
    fn test_dpms_destroy_and_live_disable_wake() {
        let mut dpms = DpmsController::new();
        dpms.set_timeout_secs(300);
        dpms.on_compositor_mode(OutputPowerMode::Off);
        assert!(dpms.is_sleeping());

        // Extending timeout above elapsed time wakes sleeping outputs
        assert!(dpms.check_idle_sleep(Duration::from_mins(2)));
        assert!(!dpms.is_sleeping());

        // Disabling timeout (0) while sleeping wakes outputs
        dpms.on_compositor_mode(OutputPowerMode::Off);
        dpms.set_timeout_secs(0);
        assert!(dpms.check_idle_sleep(Duration::from_mins(15)));
        assert!(!dpms.is_sleeping());

        dpms.on_compositor_mode(OutputPowerMode::Off);
        assert!(dpms.is_sleeping());
        dpms.destroy();
        assert!(!dpms.is_sleeping());
    }
}
