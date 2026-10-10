// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use std::time::Duration;

use cosmic::Application;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::futures::SinkExt;
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{Limits, Subscription, futures, time, window::Id};
use cosmic::prelude::*;

use crate::fl;

use super::{AppModel, Message};

impl AppModel {
    pub(crate) fn handle_update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::Surface(a) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(a),
                ));
            }
            Message::SubscriptionChannel => {}
            Message::Refresh => {
                self.refresh_daemon_state();
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::ToggleAdvanced => {
                self.show_advanced = !self.show_advanced;
            }
            Message::ToggleDaemon(toggled) => {
                if toggled {
                    match crate::daemon_client::start_daemon_service() {
                        Ok(()) => {
                            self.daemon_running = true;
                            self.last_error = None;
                        }
                        Err(e) => {
                            idle_log::error!("failed to start idle-daemon: {e:#}");
                            self.last_error = Some(fl!("error-start"));
                            self.daemon_running = crate::daemon_client::is_running();
                        }
                    }
                    self.refresh_daemon_state();
                } else if let Err(e) = crate::daemon_client::stop_daemon_service() {
                    idle_log::error!("failed to stop idle-daemon: {e:#}");
                    self.last_error = Some(fl!("error-stop"));
                    self.daemon_running = crate::daemon_client::is_running();
                } else {
                    self.daemon_running = crate::daemon_client::is_running();
                    self.last_error = None;
                }
            }
            Message::OpenDashboard => {
                // idle-tui needs a TTY — launch inside a terminal emulator.
                if crate::daemon_client::open_tui_dashboard() {
                    self.last_error = None;
                    // Close popup so the new terminal isn't hidden under it.
                    if let Some(p) = self.popup.take() {
                        return destroy_popup(p);
                    }
                } else {
                    self.last_error = Some(fl!("error-tui"));
                }
            }
            Message::ToggleIdleEnabled(toggled) => {
                self.local_config.idle_enabled = toggled;
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_idle_enabled(toggled);
                } else {
                    let _ = self.local_config.save_field("idle_enabled");
                }
            }
            Message::ToggleFpsOverlay(toggled) => {
                self.show_fps_overlay = toggled;
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_show_fps_overlay(toggled);
                } else {
                    self.local_config.show_fps_overlay = toggled;
                    let _ = self.local_config.save_field("show_fps_overlay");
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
                    let _ = self.local_config.save_field("active_saver");
                }
            }
            Message::DecreaseTimeout => {
                self.bump_timeout(-5);
            }
            Message::IncreaseTimeout => {
                self.bump_timeout(5);
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    self.refresh_daemon_state();
                    self.last_error = None;

                    if let Some(main_win_id) = self.core.main_window_id() {
                        let new_id = Id::unique();
                        self.popup.replace(new_id);
                        let mut popup_settings = self.core.applet.get_popup_settings(
                            main_win_id,
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
                    } else {
                        Task::none()
                    }
                };
            }
            Message::MiddleClick => {
                let saver = self.pick_preview_saver(/* random_if_unset */ true);
                if let Err(e) = crate::daemon_client::preview_saver(&saver) {
                    idle_log::error!("preview failed for '{saver}': {e:#}");
                    self.last_error = Some(fl!("error-preview"));
                } else {
                    self.last_error = None;
                }
            }
            Message::TriggerPreview => {
                let saver = self.pick_preview_saver(/* random_if_unset */ false);
                if let Err(e) = crate::daemon_client::preview_saver(&saver) {
                    idle_log::error!("preview failed for '{saver}': {e:#}");
                    self.last_error = Some(fl!("error-preview"));
                } else {
                    self.last_error = None;
                }
            }
            Message::ChangeRenderScale(scale) => {
                self.local_config.render_scale = Some(scale);
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_render_scale(scale);
                } else {
                    let _ = self.local_config.save_field("render_scale");
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
        let mut subs = vec![
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
        ];
        // Refresh live state while the popup is open.
        if self.popup.is_some() {
            subs.push(time::every(Duration::from_secs(2)).map(|_| Message::Refresh));
        }
        Subscription::batch(subs)
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
            screensavers: idle_runner::discovery::detect_screensavers(),
            daemon_running: false,
            show_fps_overlay: false,
            show_advanced: false,
            last_error: None,
            popup: None,
        };
        app.refresh_daemon_state();
        (app, Task::none())
    }
}
