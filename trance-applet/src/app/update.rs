// SPDX-License-Identifier: MIT

use cosmic::Application;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::futures::SinkExt;
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{Limits, Subscription, futures, window::Id};
use cosmic::prelude::*;

use super::{AppModel, Message};

impl AppModel {
    #[tracing::instrument(skip(self, message), level = "debug")]
    pub(crate) fn handle_update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::SubscriptionChannel => {}
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::ToggleDaemon(toggled) => {
                self.daemon_running = toggled;
                if toggled {
                    let sys_status = std::process::Command::new("systemctl")
                        .args(["--user", "start", "trance-daemon"])
                        .status();
                    let success = sys_status.map(|s| s.success()).unwrap_or(false);
                    if !success {
                        let _ = std::process::Command::new("trance-daemon")
                            .arg("daemon")
                            .spawn();
                    }
                } else {
                    let sys_status = std::process::Command::new("systemctl")
                        .args(["--user", "stop", "trance-daemon"])
                        .status();
                    let success = sys_status.map(|s| s.success()).unwrap_or(false);
                    if !success {
                        let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
                            std::path::PathBuf::from(runtime_dir).join("trance-daemon.pid")
                        } else {
                            std::env::temp_dir().join("trance-daemon.pid")
                        };
                        if let Ok(pid_str) = std::fs::read_to_string(&pid_path)
                            && let Ok(pid) = pid_str.trim().parse::<i32>()
                        {
                            unsafe {
                                libc::kill(pid, libc::SIGTERM);
                            }
                        }
                    }
                }
            }
            Message::OpenPowerSettings => {
                let _ = std::process::Command::new("cosmic-settings")
                    .arg("power")
                    .spawn();
            }
            Message::ToggleIdleEnabled(toggled) => {
                self.local_config.idle_enabled = toggled;
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_idle_enabled(toggled);
                } else {
                    let _ = self.local_config.save();
                }
            }
            Message::ToggleFpsOverlay(toggled) => {
                self.show_fps_overlay = toggled;
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_show_fps_overlay(toggled);
                } else {
                    self.local_config.show_fps_overlay = toggled;
                    let _ = self.local_config.save();
                }
            }

            Message::ActiveSaverSelected(saver) => {
                if saver == "Random" {
                    self.local_config.active_saver = None;
                } else {
                    self.local_config.active_saver = Some(saver);
                }
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_active_saver(
                        self.local_config.active_saver.as_deref(),
                    );
                } else {
                    let _ = self.local_config.save();
                }
            }
            Message::DecreaseTimeout => {
                if self.local_config.idle_timeout_mins > 1 {
                    self.local_config.idle_timeout_mins -= 1;
                    if crate::daemon_client::is_running() {
                        let _ =
                            crate::daemon_client::set_timeout(self.local_config.idle_timeout_mins);
                    } else {
                        let _ = self.local_config.save();
                    }
                }
            }
            Message::IncreaseTimeout => {
                if self.local_config.idle_timeout_mins < 120 {
                    self.local_config.idle_timeout_mins += 1;
                    if crate::daemon_client::is_running() {
                        let _ =
                            crate::daemon_client::set_timeout(self.local_config.idle_timeout_mins);
                    } else {
                        let _ = self.local_config.save();
                    }
                }
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    self.refresh_daemon_state();

                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                };
            }
            Message::MiddleClick => {
                let saver = self.local_config.active_saver.clone().unwrap_or_else(|| {
                    if self.screensavers.is_empty() {
                        "beams".to_string()
                    } else {
                        let idx = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as usize
                            % self.screensavers.len();
                        self.screensavers[idx].clone()
                    }
                });
                let mut started_via_dbus = false;
                if crate::daemon_client::is_running()
                    && crate::daemon_client::start_preview(&saver).is_ok()
                {
                    started_via_dbus = true;
                }
                if !started_via_dbus {
                    let _ = std::process::Command::new("trance-runner")
                        .args(["preview", &saver])
                        .spawn();
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    pub(crate) fn subscription_batch(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            Subscription::run(|| {
                cosmic::iced::stream::channel(
                    4,
                    move |mut channel: futures::channel::mpsc::Sender<_>| async move {
                        _ = channel.send(Message::SubscriptionChannel).await;
                        futures::future::pending().await
                    },
                )
            }),
            self.core()
                .watch_config::<crate::config::Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    pub(crate) fn init_app(core: cosmic::Core) -> (Self, Task<cosmic::Action<Message>>) {
        let mut app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, crate::config::Config::VERSION)
                .map(|context| match crate::config::Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            local_config: crate::config::ThemeConfig::load(),
            screensavers: trance_runner::discovery::detect_screensavers(),
            daemon_running: false,
            gpu_enabled: false,
            show_fps_overlay: false,
            popup: None,
        };
        app.refresh_daemon_state();
        (app, Task::none())
    }
}
