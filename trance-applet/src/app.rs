// SPDX-License-Identifier: MIT

use crate::config::Local76Config;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{Limits, Subscription, futures, window::Id};
use cosmic::prelude::*;
use cosmic::widget;
use futures::SinkExt;

pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: crate::config::Config,
    local_config: Local76Config,
    screensavers: Vec<String>,
    daemon_running: bool,
    gpu_enabled: bool,
    show_fps_overlay: bool,
    display_mode: String,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: cosmic::Core::default(),
            popup: None,
            config: crate::config::Config::default(),
            local_config: Local76Config::default(),
            screensavers: Vec::new(),
            daemon_running: false,
            gpu_enabled: true,
            show_fps_overlay: false,
            display_mode: "primary".to_string(),
        }
    }
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SubscriptionChannel,
    UpdateConfig(crate::config::Config),
    ToggleIdleEnabled(bool),
    ActiveSaverSelected(String),
    ToggleDaemon(bool),
    ToggleGpuEnabled(bool),
    ToggleFpsOverlay(bool),
    DecreaseTimeout,
    IncreaseTimeout,
    OpenPowerSettings,
    DisplayModeSelected(String),
}

impl AppModel {
    fn refresh_daemon_state(&mut self) {
        self.daemon_running = crate::daemon_client::is_running();
        if self.daemon_running {
            if let Some(status) = crate::daemon_client::fetch_status() {
                self.local_config.idle_enabled = status.idle_enabled;
                self.local_config.idle_timeout_mins = status.idle_timeout_mins;
                self.local_config.active_saver = if status.active_saver.is_empty() {
                    None
                } else {
                    Some(status.active_saver)
                };
                self.gpu_enabled = status.gpu_enabled;
                self.show_fps_overlay = status.show_fps_overlay;
                self.display_mode = if status.display_mode.is_empty() {
                    "primary".to_string()
                } else {
                    status.display_mode
                };
            }
            if let Ok(savers) = crate::daemon_client::list_savers() {
                self.screensavers = savers;
            }
        } else {
            self.local_config = Local76Config::load();
            self.screensavers = trance_runner::discovery::detect_screensavers();
            self.gpu_enabled = self.local_config.gpu_enabled;
            self.show_fps_overlay = self.local_config.show_fps_overlay;
        }
    }
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.system76.CosmicApplet.Trance";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, crate::config::Config::VERSION)
                .map(|context| match crate::config::Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            local_config: Local76Config::load(),
            screensavers: trance_runner::discovery::detect_screensavers(),
            daemon_running: false,
            gpu_enabled: true,
            show_fps_overlay: false,
            display_mode: "primary".to_string(),
            popup: None,
        };
        app.refresh_daemon_state();

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// The applet's button in the panel will be drawn using the main view method.
    /// This view should emit messages to toggle the applet's popup window, which will
    /// be drawn using the `view_window` method.
    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("display-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    /// The applet's popup window will be drawn using this view method. If there are
    /// multiple poups, you may match the id parameter to determine which popup to
    /// create a view for.
    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let options = {
            let mut opts = vec!["Random".to_string()];
            for s in &self.screensavers {
                opts.push(s.clone());
            }
            opts
        };
        let selected = Some(
            self.local_config
                .active_saver
                .clone()
                .unwrap_or_else(|| "Random".to_string()),
        );

        let mut grid = cosmic::iced::widget::Column::new()
            .spacing(6)
            .width(cosmic::iced::Length::Fill);
        let mut row = cosmic::iced::widget::Row::new()
            .spacing(6)
            .width(cosmic::iced::Length::Fill);
        let len = options.len();
        for (i, s) in options.into_iter().enumerate() {
            let is_selected = selected.as_ref() == Some(&s);
            let btn = if is_selected {
                widget::button::suggested(s.clone())
            } else {
                widget::button::standard(s.clone())
            };
            let btn = btn
                .width(cosmic::iced::Length::Fill)
                .on_press(Message::ActiveSaverSelected(s));
            row = row.push(btn);
            if i % 2 == 1 {
                grid = grid.push(row);
                row = cosmic::iced::widget::Row::new()
                    .spacing(6)
                    .width(cosmic::iced::Length::Fill);
            }
        }
        if len % 2 != 0 {
            grid = grid.push(row);
        }

        let header = widget::text("Trance Screensaver").size(16);

        let decrease_btn = widget::button::standard("-").on_press(Message::DecreaseTimeout);
        let increase_btn = widget::button::standard("+").on_press(Message::IncreaseTimeout);
        let timeout_val = widget::text(format!("{} mins", self.local_config.idle_timeout_mins));

        let timeout_adjuster = cosmic::iced::widget::Row::new()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .push(decrease_btn)
            .push(timeout_val)
            .push(increase_btn);

        let actions = widget::button::standard("Power Settings")
            .width(cosmic::iced::Length::Fill)
            .on_press(Message::OpenPowerSettings);

        let content_list = widget::list_column()
            .add(header)
            .add(widget::settings::item(
                "Background Daemon",
                widget::toggler(self.daemon_running).on_toggle(Message::ToggleDaemon),
            ))
            .add(widget::settings::item(
                "Idle Activation",
                widget::toggler(self.local_config.idle_enabled)
                    .on_toggle(Message::ToggleIdleEnabled),
            ))
            .add(widget::settings::item(
                "GPU Upscaling",
                widget::toggler(self.gpu_enabled).on_toggle(Message::ToggleGpuEnabled),
            ))
            .add(widget::settings::item(
                "FPS Overlay",
                widget::toggler(self.show_fps_overlay).on_toggle(Message::ToggleFpsOverlay),
            ))
            .add(widget::settings::item(
                "Display Mode",
                cosmic::iced::widget::Row::new()
                    .spacing(6)
                    .push(
                        widget::button::standard("Primary")
                            .on_press(Message::DisplayModeSelected("primary".into())),
                    )
                    .push(
                        widget::button::standard("Mirror")
                            .on_press(Message::DisplayModeSelected("mirror".into())),
                    )
                    .push(
                        widget::button::standard("Expand")
                            .on_press(Message::DisplayModeSelected("expand".into())),
                    ),
            ))
            .add(widget::settings::item(
                "Active Layout",
                widget::text(self.display_mode.clone()),
            ))
            .add(widget::settings::item("Idle Timeout", timeout_adjuster))
            .add(grid)
            .add(actions);

        self.core.applet.popup_container(content_list).into()
    }

    /// Register subscriptions for this application.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run(|| {
                cosmic::iced::stream::channel(
                    4,
                    move |mut channel: futures::channel::mpsc::Sender<_>| async move {
                        _ = channel.send(Message::SubscriptionChannel).await;
                        futures::future::pending().await
                    },
                )
            }),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<crate::config::Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
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
                        if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                                unsafe {
                                    libc::kill(pid, libc::SIGTERM);
                                }
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
            Message::ToggleGpuEnabled(toggled) => {
                self.gpu_enabled = toggled;
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_gpu_enabled(toggled);
                } else {
                    self.local_config.gpu_enabled = toggled;
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
            Message::DisplayModeSelected(mode) => {
                self.display_mode = mode.clone();
                self.local_config.display_mode = mode.clone();
                if crate::daemon_client::is_running() {
                    let _ = crate::daemon_client::set_display_mode(&mode);
                } else {
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
                        let _ = crate::daemon_client::set_timeout(
                            self.local_config.idle_timeout_mins,
                        );
                    } else {
                        let _ = self.local_config.save();
                    }
                }
            }
            Message::IncreaseTimeout => {
                if self.local_config.idle_timeout_mins < 120 {
                    self.local_config.idle_timeout_mins += 1;
                    if crate::daemon_client::is_running() {
                        let _ = crate::daemon_client::set_timeout(
                            self.local_config.idle_timeout_mins,
                        );
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
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
