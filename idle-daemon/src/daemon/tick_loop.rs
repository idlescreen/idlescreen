// SPDX-License-Identifier: MIT

//! Main idle-detection tick loop and per-tick command dispatch.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::idle_logic::update_presentation_state;
use super::presentation::ActivePresentation;
use super::runtime::check_runtime_alive;
use crate::controller::{DaemonCommand, DaemonController, MAIN_LOOP_INTERVAL};

pub fn tick_loop_until_shutdown(controller: Arc<DaemonController>) -> anyhow::Result<()> {
    let (mut idle_monitor, overlay_presenter) = super::runtime::initialize_runtime(&controller)?;
    let mut presentation = ActivePresentation::None;
    let mut preview_name: Option<String> = None;
    let mut current_saver = String::new();
    let mut tick_counter = 0u32;

    while !controller.shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(MAIN_LOOP_INTERVAL);
        tick_counter = tick_counter.saturating_add(1);

        check_runtime_alive(&idle_monitor, &overlay_presenter)?;
        dispatch_tick_commands(
            &controller,
            &overlay_presenter,
            &mut idle_monitor,
            &mut presentation,
            &mut preview_name,
            &mut current_saver,
        );
        if let Some(timeout) = controller.reload_config_if_due(tick_counter) {
            idle_monitor.set_timeout(timeout);
        }

        // One config snapshot per tick for presentation decisions.
        let config = controller
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let system_idle = idle_monitor.is_idle();
        let session_locked = controller.session_locked.load(Ordering::Relaxed);
        let mut inhibited = controller.inhibitors.is_inhibited();

        if is_on_battery() {
            inhibited = true;
        }

        update_presentation_state(
            &overlay_presenter,
            &mut presentation,
            &mut preview_name,
            &mut current_saver,
            &config,
            system_idle,
            session_locked,
            inhibited,
        );

        // Reuses a fresh config lock inside update_live_state (status may need
        // values that changed via D-Bus mid-tick).
        controller.update_live_state(
            system_idle,
            presentation.is_active(),
            preview_name.is_some(),
            &current_saver,
        );
        controller.publish_status_if_dirty();
    }

    super::presentation::stop_presentation(Some(&overlay_presenter), &mut presentation);
    Ok(())
}

fn dispatch_tick_commands(
    controller: &DaemonController,
    overlay_presenter: &Arc<wayland_present::OverlayPresenter>,
    idle_monitor: &mut wayland_idle::IdleMonitor,
    presentation: &mut ActivePresentation,
    preview_name: &mut Option<String>,
    current_saver: &mut String,
) {
    for command in controller.drain_commands() {
        match command {
            DaemonCommand::Preview(name) => {
                *preview_name = Some(name);
            }
            DaemonCommand::StopPresentation => {
                *preview_name = None;
                super::presentation::stop_presentation(Some(overlay_presenter), presentation);
                current_saver.clear();
            }
            DaemonCommand::SetTimeout(minutes) => {
                let _ = controller.apply_command(DaemonCommand::SetTimeout(minutes));
                idle_monitor.set_timeout(minutes);
            }
            DaemonCommand::Enable
            | DaemonCommand::Disable
            | DaemonCommand::SetSaver(_)
            | DaemonCommand::SetShowFpsOverlay(_)
            | DaemonCommand::SetRenderScale(_) => {
                let _ = controller.apply_command(command);
            }
        }
    }
}

fn is_on_battery() -> bool {
    let path = std::path::Path::new("/sys/class/power_supply");
    if let Ok(entries) = std::fs::read_dir(path) {
        let mut has_ac = false;
        let mut ac_online = true;
        let mut battery_discharging = false;

        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(t) = std::fs::read_to_string(p.join("type")) {
                let type_str = t.trim();
                if type_str == "Mains" {
                    has_ac = true;
                    if let Ok(o) = std::fs::read_to_string(p.join("online")) {
                        if o.trim() == "0" {
                            ac_online = false;
                        } else {
                            ac_online = true;
                        }
                    }
                } else if type_str == "Battery" {
                    if let Ok(s) = std::fs::read_to_string(p.join("status")) {
                        if s.trim() == "Discharging" {
                            battery_discharging = true;
                        }
                    }
                }
            }
        }

        // If AC is present but offline, or a battery is discharging, we're on battery
        if (has_ac && !ac_online) || battery_discharging {
            return true;
        }
    }
    false
}
