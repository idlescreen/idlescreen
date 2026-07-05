// SPDX-License-Identifier: MIT

//! Idle-driven screensaver presentation state machine.

use std::sync::Arc;

use wayland_present::OverlayPresenter;

use super::presentation::{
    ActivePresentation, current_time_micros, pick_saver_name, start_presentation, stop_presentation,
};
use crate::config::DaemonConfig;

pub fn update_presentation_state(
    overlay_presenter: &Arc<OverlayPresenter>,
    presentation: &mut ActivePresentation,
    preview_name: &mut Option<String>,
    current_saver: &mut String,
    config: &DaemonConfig,
    system_idle: bool,
    session_locked: bool,
    inhibited: bool,
) {
    clear_stale_presentation(overlay_presenter, presentation, preview_name, current_saver);
    drive_presentation_chain(
        overlay_presenter,
        presentation,
        preview_name,
        current_saver,
        config,
        system_idle,
        session_locked,
        inhibited,
    );
}

fn clear_stale_presentation(
    overlay_presenter: &Arc<OverlayPresenter>,
    presentation: &mut ActivePresentation,
    preview_name: &mut Option<String>,
    current_saver: &mut String,
) {
    if presentation.is_active() && !overlay_presenter.is_visible() {
        stop_presentation(Some(overlay_presenter), presentation);
        current_saver.clear();
        *preview_name = None;
    }
}

fn drive_presentation_chain(
    overlay_presenter: &Arc<OverlayPresenter>,
    presentation: &mut ActivePresentation,
    preview_name: &mut Option<String>,
    current_saver: &mut String,
    config: &DaemonConfig,
    system_idle: bool,
    session_locked: bool,
    inhibited: bool,
) {
    if session_locked || inhibited {
        if presentation.is_active() {
            stop_presentation(Some(overlay_presenter), presentation);
            current_saver.clear();
        }
        *preview_name = None;
    } else if let Some(name) = preview_name.clone() {
        if !presentation.is_active() {
            start_presentation(
                overlay_presenter,
                presentation,
                current_saver,
                name,
                "preview",
                config,
            );
        }
    } else if config.idle_enabled && system_idle && !presentation.is_active() {
        let seed_micros = current_time_micros();
        let saver_name = pick_saver_name(config, seed_micros);
        start_presentation(
            overlay_presenter,
            presentation,
            current_saver,
            saver_name,
            "idle",
            config,
        );
    } else if presentation.is_active() && !system_idle && preview_name.is_none() {
        stop_presentation(Some(overlay_presenter), presentation);
        current_saver.clear();
        tracing::info!("system activity detected. presentation stopped.");
    } else if !config.idle_enabled && presentation.is_active() {
        stop_presentation(Some(overlay_presenter), presentation);
        current_saver.clear();
    }
}
