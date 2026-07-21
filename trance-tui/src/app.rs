// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

use std::process::Command;
use std::time::Duration;
use trance_dbus::{TranceClient, daemon_available};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ActivePane {
    Settings,
    Screensavers,
}

pub struct App {
    pub client: Option<TranceClient>,
    pub daemon_running: bool,
    pub idle_enabled: bool,
    pub idle_timeout_mins: u32,
    pub render_scale: f32,
    pub show_fps_overlay: bool,
    pub active_saver: String,
    pub on_battery: bool,
    pub screensavers: Vec<String>,
    pub selected_saver_idx: usize,
    pub active_pane: ActivePane,
    pub selected_setting_idx: usize,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            client: None,
            daemon_running: false,
            idle_enabled: true,
            idle_timeout_mins: 5,
            render_scale: 1.0,
            show_fps_overlay: false,
            active_saver: "Random".to_string(),
            on_battery: false,
            screensavers: Vec::new(),
            selected_saver_idx: 0,
            active_pane: ActivePane::Settings,
            selected_setting_idx: 0,
        };
        app.refresh_state();
        app
    }

    pub fn refresh_state(&mut self) {
        self.daemon_running = daemon_available();
        if self.daemon_running {
            if let Ok(client) = TranceClient::connect() {
                if let Ok(status) = client.get_status() {
                    self.idle_enabled = status.idle_enabled;
                    self.idle_timeout_mins = status.idle_timeout_mins;
                    self.active_saver = if status.active_saver.is_empty() {
                        "Random".to_string()
                    } else {
                        status.active_saver
                    };
                    self.show_fps_overlay = status.show_fps_overlay;
                    self.render_scale = status.render_scale.parse::<f32>().unwrap_or(1.0);
                    self.on_battery = status.inhibited;
                }
                if let Ok(savers) = client.list_savers() {
                    self.screensavers = savers;
                }
                self.client = Some(client);
            }
        } else {
            self.client = None;
            self.screensavers = trance_runner::discovery::detect_screensavers();
        }

        let sys = trance_runner::toolkit::sys_info::get_system_info();
        self.on_battery = sys.power_status.contains("Battery");
    }

    pub fn toggle_daemon(&mut self) {
        if self.daemon_running {
            let _ = Command::new("systemctl")
                .args(["--user", "stop", "trance-daemon.service"])
                .status();
        } else {
            let sys_status = Command::new("systemctl")
                .args(["--user", "enable", "--now", "trance-daemon.service"])
                .status();
            let success = sys_status.map(|s| s.success()).unwrap_or(false);
            if !success {
                let _ = Command::new("trance-daemon").arg("daemon").spawn();
            }
        }
        std::thread::sleep(Duration::from_millis(350));
        self.refresh_state();
    }

    pub fn toggle_idle(&mut self) {
        if let Some(ref client) = self.client {
            if self.idle_enabled {
                let _ = client.disable();
            } else {
                let _ = client.enable();
            }
        }
        self.refresh_state();
    }

    pub fn adjust_timeout(&mut self, delta: i32) {
        let mut val = self.idle_timeout_mins as i32 + delta;
        val = val.clamp(1, 240);
        self.idle_timeout_mins = val as u32;
        if let Some(ref client) = self.client {
            let _ = client.set_timeout(self.idle_timeout_mins);
        }
    }

    pub fn adjust_scale(&mut self, delta: f32) {
        let mut val = self.render_scale + delta;
        val = val.clamp(0.25, 1.0);
        self.render_scale = val;
        if let Some(ref client) = self.client {
            let _ = client.set_render_scale(self.render_scale);
        }
    }

    pub fn toggle_fps(&mut self) {
        if let Some(ref client) = self.client {
            let _ = client.set_show_fps_overlay(!self.show_fps_overlay);
        }
        self.refresh_state();
    }

    pub fn select_saver(&mut self) {
        if let Some(ref client) = self.client {
            let name = if self.selected_saver_idx == 0 {
                ""
            } else {
                &self.screensavers[self.selected_saver_idx - 1]
            };
            let _ = client.set_saver(name);
        }
        self.refresh_state();
    }

    pub fn preview_saver(&mut self) {
        let saver = if self.selected_saver_idx == 0 {
            if self.screensavers.is_empty() {
                "beams".to_string()
            } else {
                self.screensavers[0].clone()
            }
        } else {
            self.screensavers[self.selected_saver_idx - 1].clone()
        };

        if !self.daemon_running {
            self.toggle_daemon();
        }

        let mut started_via_dbus = false;
        if self.daemon_running {
            if self.client.is_none() {
                self.refresh_state();
            }
            if let Some(ref client) = self.client
                && client.preview(&saver).is_ok()
            {
                started_via_dbus = true;
            }
        }
        if !started_via_dbus {
            let _ = Command::new("trance-daemon")
                .args(["run-plugin", &saver])
                .status();
        }
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
