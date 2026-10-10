// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Small helper methods used by `AppModel::handle_update`. Extracted to keep
//! `update.rs` under the 256-line cap.

use super::AppModel;

impl AppModel {
    pub(crate) fn bump_timeout(&mut self, delta: i32) {
        let cur = self.local_config.idle_timeout_mins as i32;
        let next = (cur + delta).clamp(1, 120) as u32;
        if next == self.local_config.idle_timeout_mins {
            return;
        }
        self.local_config.idle_timeout_mins = next;
        if crate::daemon_client::is_running() {
            let _ = crate::daemon_client::set_timeout(next);
        } else {
            let _ = self.local_config.save_field("idle_timeout_mins");
        }
    }

    /// Choose which saver to preview: configured active, else first/random from list.
    pub(crate) fn pick_preview_saver(&self, random_if_unset: bool) -> String {
        if let Some(name) = self.local_config.active_saver.clone() {
            return name;
        }
        if self.screensavers.is_empty() {
            return "ascii".to_string();
        }
        if random_if_unset {
            let idx = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as usize
                % self.screensavers.len();
            self.screensavers[idx].clone()
        } else {
            self.screensavers[0].clone()
        }
    }
}
