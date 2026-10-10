// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use cosmic::iced::window::Id;
use cosmic::widget;
use cosmic::widget::icon;

use crate::fl;

use super::{AppModel, Message};

/// Panel glyph (monitor + idle moon). Embedded so the top-bar icon always paints.
const PANEL_ICON_SVG: &[u8] =
    include_bytes!("../../resources/io.github.idlescreen.CosmicApplet-symbolic.svg");

/// Saver buttons per row (compact popup).
const SAVER_COLS: usize = 4;

impl AppModel {
    pub(crate) fn view_panel(&self) -> cosmic::Element<'_, Message> {
        let handle = icon::from_svg_bytes(PANEL_ICON_SVG).symbolic(true);
        let btn = self
            .core
            .applet
            .icon_button_from_handle(handle)
            .on_press(Message::TogglePopup);

        let btn = cosmic::iced::widget::mouse_area(btn).on_middle_press(Message::MiddleClick);

        self.core
            .applet
            .applet_tooltip(
                btn,
                fl!("tooltip"),
                self.popup.is_some(),
                Message::Surface,
                self.core.main_window_id(),
            )
            .into()
    }

    pub(crate) fn view_popup(&self, _id: Id) -> cosmic::Element<'_, Message> {
        // Header: title + Preview (standard style — not suggested/accent red).
        let preview_label = if self.daemon_running {
            fl!("preview-now")
        } else {
            fl!("preview-starts-daemon")
        };
        let preview_btn = widget::button::standard(preview_label).on_press(Message::TriggerPreview);

        let mut header = cosmic::iced::widget::Column::new().spacing(2).push(
            cosmic::iced::widget::Row::new()
                .spacing(8)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text(fl!("app-title")).size(15))
                .push(cosmic::iced::widget::Space::new().width(cosmic::iced::Length::Fill))
                .push(preview_btn),
        );
        if let Some(err) = &self.last_error {
            header = header.push(widget::text(err.as_str()).size(11));
        }

        let decrease_btn = widget::button::standard("−").on_press(Message::DecreaseTimeout);
        let increase_btn = widget::button::standard("+").on_press(Message::IncreaseTimeout);
        let timeout_val = widget::text(fl!(
            "timeout-minutes",
            mins = self.local_config.idle_timeout_mins
        ));
        let timeout_adjuster = cosmic::iced::widget::Row::new()
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center)
            .push(decrease_btn)
            .push(timeout_val)
            .push(increase_btn);

        // Main: activation, timeout, savers, advanced (Preview is in the header).
        let mut content_list = widget::list_column()
            .add(header)
            .add(widget::settings::item(
                fl!("idle-activation"),
                widget::toggler(self.local_config.idle_enabled)
                    .on_toggle(Message::ToggleIdleEnabled),
            ))
            .add(widget::settings::item(
                fl!("idle-timeout"),
                timeout_adjuster,
            ))
            .add(self.saver_grid())
            .add(
                widget::button::standard(fl!("advanced"))
                    .width(cosmic::iced::Length::Fill)
                    .on_press(Message::ToggleAdvanced),
            );

        if self.show_advanced {
            let scale_val = widget::text(fl!(
                "scale-percent",
                pct = ((self.local_config.render_scale.unwrap_or(1.0) * 100.0).round() as u32)
            ));
            let scale_slider = cosmic::iced::widget::Slider::new(
                0.25..=1.0,
                self.local_config.render_scale.unwrap_or(1.0),
                Message::ChangeRenderScale,
            )
            .step(0.05_f32);
            let scale_adjuster = cosmic::iced::widget::Row::new()
                .spacing(6)
                .align_y(cosmic::iced::Alignment::Center)
                .push(scale_slider)
                .push(scale_val);

            content_list = content_list
                .add(widget::settings::item(fl!("render-scale"), scale_adjuster))
                .add(widget::settings::item(
                    fl!("fps-overlay"),
                    widget::toggler(self.show_fps_overlay).on_toggle(Message::ToggleFpsOverlay),
                ))
                .add(widget::settings::item(
                    fl!("daemon-service"),
                    widget::toggler(self.daemon_running).on_toggle(Message::ToggleDaemon),
                ))
                .add(
                    widget::button::standard(fl!("open-dashboard"))
                        .width(cosmic::iced::Length::Fill)
                        .on_press(Message::OpenDashboard),
                );
        }

        self.core.applet.popup_container(content_list).into()
    }

    fn saver_grid(&self) -> cosmic::Element<'_, Message> {
        if self.screensavers.is_empty() {
            return cosmic::iced::widget::container(widget::text(fl!("no-savers")).size(12))
                .width(cosmic::iced::Length::Fill)
                .padding(6)
                .into();
        }

        let mut options = vec!["Random".to_string()];
        options.extend(self.screensavers.iter().cloned());

        let selected = self
            .local_config
            .active_saver
            .clone()
            .unwrap_or_else(|| "Random".to_string());

        let mut grid = cosmic::iced::widget::Column::new()
            .spacing(4)
            .width(cosmic::iced::Length::Fill);
        let mut row = cosmic::iced::widget::Row::new()
            .spacing(4)
            .width(cosmic::iced::Length::Fill);
        let len = options.len();

        for (i, s) in options.into_iter().enumerate() {
            let is_selected = selected == s;
            let label = display_saver_name(&s);
            let btn = if is_selected {
                widget::button::suggested(label)
            } else {
                widget::button::standard(label)
            };
            row = row.push(
                btn.width(cosmic::iced::Length::Fill)
                    .on_press(Message::ActiveSaverSelected(s)),
            );
            if (i + 1) % SAVER_COLS == 0 {
                grid = grid.push(row);
                row = cosmic::iced::widget::Row::new()
                    .spacing(4)
                    .width(cosmic::iced::Length::Fill);
            }
        }
        if len % SAVER_COLS != 0 {
            grid = grid.push(row);
        }

        // 4 columns → ~3 rows for 11 items; keep height modest.
        cosmic::iced::widget::scrollable(grid).height(120.0).into()
    }
}

fn display_saver_name(raw: &str) -> String {
    if raw.eq_ignore_ascii_case("random") {
        return fl!("random");
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

#[cfg(test)]
mod tests {
    use super::display_saver_name;

    #[test]
    fn title_cases_savers() {
        assert_eq!(display_saver_name("beams"), "Beams");
        assert_eq!(display_saver_name("my_cool-saver"), "My Cool Saver");
    }
}
