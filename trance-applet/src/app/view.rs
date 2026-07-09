// SPDX-License-Identifier: MIT

use cosmic::iced::window::Id;
use cosmic::widget;

use super::{AppModel, Message};

impl AppModel {
    pub(crate) fn view_panel(&self) -> cosmic::Element<'_, Message> {
        let btn = self
            .core
            .applet
            .icon_button("display-symbolic")
            .on_press(Message::TogglePopup);
        // Middle-click = quick preview of the active (or random) saver.
        cosmic::iced::widget::mouse_area(btn)
            .on_middle_press(Message::MiddleClick)
            .into()
    }

    pub(crate) fn view_popup(&self, _id: Id) -> cosmic::Element<'_, Message> {
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
            let label = display_saver_name(&s);
            let btn = if is_selected {
                widget::button::suggested(label)
            } else {
                widget::button::standard(label)
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

        // Limit the plugin list container height and make it scrollable to avoid pop-up overflow
        let scrollable_grid = cosmic::iced::widget::scrollable(grid).height(130.0);

        let on_battery = trance_runner::toolkit::sys_info::get_system_info()
            .power_status
            .contains("Battery");

        let mut header_col = cosmic::iced::widget::Column::new().spacing(2);
        let mut header_row = cosmic::iced::widget::Row::new()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .push(widget::text("Trance Screensaver").size(16));

        if on_battery {
            header_row = header_row.push(widget::text("(Battery · 30 FPS)").size(11));
        }
        header_col = header_col.push(header_row);

        // Live status so offline vs running is obvious without reading the toggle alone.
        let status_line = if self.daemon_running {
            "Daemon running · settings apply live"
        } else {
            "Daemon stopped · toggle on to start (auto-starts at login)"
        };
        header_col = header_col.push(widget::text(status_line).size(11));

        let decrease_btn = widget::button::standard("−").on_press(Message::DecreaseTimeout);
        let increase_btn = widget::button::standard("+").on_press(Message::IncreaseTimeout);
        let timeout_val = widget::text(format!("{} min", self.local_config.idle_timeout_mins));

        let timeout_adjuster = cosmic::iced::widget::Row::new()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .push(decrease_btn)
            .push(timeout_val)
            .push(increase_btn);

        // Render scale: lower = chunkier / cheaper simulation, then upscaled.
        let scale_val = widget::text(format!("{:.0}%", self.local_config.render_scale * 100.0));
        let scale_slider = cosmic::iced::widget::Slider::new(
            0.25..=1.0,
            self.local_config.render_scale,
            Message::ChangeRenderScale,
        )
        .step(0.05);

        let scale_adjuster = cosmic::iced::widget::Row::new()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .push(scale_slider)
            .push(scale_val);

        let preview_label = if self.daemon_running {
            "Preview Now"
        } else {
            "Preview (starts daemon)"
        };
        let preview_btn = widget::button::standard(preview_label)
            .width(cosmic::iced::Length::Fill)
            .on_press(Message::TriggerPreview);

        let actions = cosmic::iced::widget::Column::new()
            .spacing(6)
            .push(preview_btn)
            .push(
                widget::button::standard("Power Settings")
                    .width(cosmic::iced::Length::Fill)
                    .on_press(Message::OpenPowerSettings),
            );

        let content_list = widget::list_column()
            .add(header_col)
            .add(widget::settings::item(
                "Daemon Service",
                widget::toggler(self.daemon_running).on_toggle(Message::ToggleDaemon),
            ))
            .add(widget::settings::item(
                "Idle Activation",
                widget::toggler(self.local_config.idle_enabled)
                    .on_toggle(Message::ToggleIdleEnabled),
            ))
            .add(widget::settings::item("Idle Timeout", timeout_adjuster))
            .add(widget::settings::item("Render Scale", scale_adjuster))
            .add(widget::settings::item(
                "FPS Overlay",
                widget::toggler(self.show_fps_overlay).on_toggle(Message::ToggleFpsOverlay),
            ))
            .add(widget::text("Screensavers").size(12))
            .add(cosmic::iced::widget::container(scrollable_grid).width(cosmic::iced::Length::Fill))
            .add(actions);

        self.core.applet.popup_container(content_list).into()
    }
}

/// Title-case saver basenames for the grid (`beams` → `Beams`).
fn display_saver_name(raw: &str) -> String {
    if raw.eq_ignore_ascii_case("random") {
        return "Random".to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut cap = true;
    for ch in raw.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
            cap = true;
            continue;
        }
        if cap {
            out.extend(ch.to_uppercase());
            cap = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}
