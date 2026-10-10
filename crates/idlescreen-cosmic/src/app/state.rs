// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use crate::config::ThemeConfig;

use super::AppModel;

impl AppModel {
    pub(crate) fn refresh_daemon_state(&mut self) {
        self.daemon_running = crate::daemon_client::is_running();
        if self.daemon_running {
            if let Ok(status) = crate::daemon_client::fetch_status() {
                self.local_config.idle_enabled = status.idle_enabled;
                self.local_config.idle_timeout_mins = status.idle_timeout_mins;
                self.local_config.active_saver = if status.active_saver.is_empty() {
                    None
                } else {
                    Some(status.active_saver)
                };
                self.show_fps_overlay = status.show_fps_overlay;
                // "null"/unparseable = auto scale → None.
                self.local_config.render_scale = status.render_scale.parse::<f32>().ok();
            }
            if let Ok(savers) = crate::daemon_client::list_savers() {
                self.screensavers = savers;
            }
        } else {
            self.local_config = ThemeConfig::load();
            self.screensavers = idle_runner::discovery::detect_screensavers();
            self.show_fps_overlay = self.local_config.show_fps_overlay;
        }
    }
}
