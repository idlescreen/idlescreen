// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use trance_runner::launcher::{ALLOWED_SAVERS, is_allowed_saver};
use wayland_present::OverlayPresenter;

use crate::config::DaemonConfig;
use crate::presentation::{PluginPresentation, PresentationOptions};

pub enum ActivePresentation {
    None,
    Plugin(PluginPresentation),
}

impl ActivePresentation {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Plugin(_))
    }
}

#[tracing::instrument(skip(overlay_presenter, presentation, current_saver, config), fields(saver_name = %saver_name, reason = %reason))]
pub fn start_presentation(
    overlay_presenter: &Arc<OverlayPresenter>,
    presentation: &mut ActivePresentation,
    current_saver: &mut String,
    saver_name: String,
    reason: &str,
    config: &DaemonConfig,
) {
    tracing::info!("starting Wayland screensaver '{saver_name}' ({reason})...");
    let launch_mode = if reason == "preview" {
        trance_runner::launcher::LaunchMode::Preview
    } else {
        trance_runner::launcher::LaunchMode::Daemon
    };
    let options = PresentationOptions {
        gpu_enabled: {
            #[allow(deprecated)]
            let v = config.gpu_enabled;
            v
        },
        show_fps_overlay: config.show_fps_overlay,
        render_scale: config.render_scale,
        launch_mode,
    };
    match PluginPresentation::start(overlay_presenter.clone(), saver_name.clone(), options) {
        Ok(plugin) => {
            *current_saver = saver_name;
            *presentation = ActivePresentation::Plugin(plugin);
        }
        Err(error) => tracing::error!("failed to start screensaver: {error}"),
    }
}

#[tracing::instrument(skip(overlay_presenter, presentation))]
pub fn stop_presentation(
    overlay_presenter: Option<&Arc<OverlayPresenter>>,
    presentation: &mut ActivePresentation,
) {
    if let ActivePresentation::Plugin(plugin) = presentation {
        if let Some(presenter) = overlay_presenter {
            plugin.stop(presenter);
        }
        *presentation = ActivePresentation::None;
    }
}

pub fn current_time_micros() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_micros() as u64,
        Err(_) => 0,
    }
}

pub fn pick_saver_name(config: &DaemonConfig, seed_micros: u64) -> String {
    if let Some(active) = config
        .active_saver
        .as_deref()
        .filter(|&s| is_allowed_saver(s))
    {
        return active.to_string();
    }

    let mut seed = seed_micros;
    seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let index = (seed % ALLOWED_SAVERS.len() as u64) as usize;
    ALLOWED_SAVERS[index].to_string()
}
