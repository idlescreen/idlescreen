// SPDX-License-Identifier: MIT

use std::sync::atomic::Ordering;

use super::DaemonController;

impl DaemonController {
    #[tracing::instrument(skip_all, fields(system_idle, presentation_active, preview_active, current_saver = %current_saver))]
    pub fn update_live_state(
        &self,
        system_idle: bool,
        presentation_active: bool,
        preview_active: bool,
        current_saver: &str,
    ) {
        let mut status = self.status.lock().unwrap();
        let changed = self.compute_dirty_fields(
            &mut status,
            system_idle,
            presentation_active,
            preview_active,
            current_saver,
        );
        self.copy_live_fields(
            &mut status,
            system_idle,
            presentation_active,
            preview_active,
            current_saver,
        );
        if changed {
            self.status_dirty.store(true, Ordering::Relaxed);
        }
    }

    #[tracing::instrument(skip_all, fields(tick_counter))]
    pub fn reload_config_if_due(&self, tick_counter: u32) -> Option<u32> {
        if !tick_counter.is_multiple_of(10) {
            return None;
        }
        let reloaded = crate::config::DaemonConfig::load();
        let mut config = self.config.lock().unwrap();
        let previous_timeout = config.idle_timeout_mins;
        if *config != reloaded {
            *config = reloaded;
            self.mark_dirty();
        }
        if config.idle_timeout_mins != previous_timeout {
            Some(config.idle_timeout_mins)
        } else {
            None
        }
    }

    fn compute_dirty_fields(
        &self,
        status: &mut trance_dbus::DaemonStatus,
        system_idle: bool,
        presentation_active: bool,
        preview_active: bool,
        current_saver: &str,
    ) -> bool {
        let config = self.config.lock().unwrap().clone();
        let session_locked = self.session_locked.load(Ordering::Relaxed);
        let inhibited = self.inhibitors.is_inhibited();

        status.system_idle != system_idle
            || status.presentation_active != presentation_active
            || status.preview_active != preview_active
            || status.session_locked != session_locked
            || status.inhibited != inhibited
            || status.idle_enabled != config.idle_enabled
            || status.idle_timeout_mins != config.idle_timeout_mins
            || status.active_saver != config.active_saver.clone().unwrap_or_default()
            || {
                #[allow(deprecated)]
                let gpu_diff = status.gpu_enabled != config.gpu_enabled;
                gpu_diff
            }
            || status.show_fps_overlay != config.show_fps_overlay
            || status.render_scale
                != config
                    .render_scale
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            || status.current_saver != current_saver
    }

    fn copy_live_fields(
        &self,
        status: &mut trance_dbus::DaemonStatus,
        system_idle: bool,
        presentation_active: bool,
        preview_active: bool,
        current_saver: &str,
    ) {
        let config = self.config.lock().unwrap().clone();
        let session_locked = self.session_locked.load(Ordering::Relaxed);
        let inhibited = self.inhibitors.is_inhibited();

        status.running = true;
        status.system_idle = system_idle;
        status.presentation_active = presentation_active;
        status.preview_active = preview_active;
        status.session_locked = session_locked;
        status.inhibited = inhibited;
        status.idle_enabled = config.idle_enabled;
        status.idle_timeout_mins = config.idle_timeout_mins;
        status.active_saver = config.active_saver.clone().unwrap_or_default();
        #[allow(deprecated)]
        {
            status.gpu_enabled = config.gpu_enabled;
        }
        status.show_fps_overlay = config.show_fps_overlay;
        status.render_scale = config
            .render_scale
            .map(|s| s.to_string())
            .unwrap_or_default();
        status.current_saver = current_saver.to_string();
    }
}
