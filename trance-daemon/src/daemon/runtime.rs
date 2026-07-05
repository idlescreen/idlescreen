// SPDX-License-Identifier: MIT

//! Wayland runtime initialization and liveness checks.

use std::sync::Arc;

use anyhow::anyhow;
use wayland_idle::IdleMonitor;
use wayland_present::OverlayPresenter;

use crate::controller::DaemonController;

pub fn initialize_runtime(
    controller: &DaemonController,
) -> anyhow::Result<(IdleMonitor, Arc<OverlayPresenter>)> {
    let idle_timeout = controller.config.lock().unwrap().idle_timeout_mins;
    let idle_monitor = IdleMonitor::new(idle_timeout).ok_or_else(|| {
        anyhow!("Wayland idle monitoring is unavailable; ensure ext-idle-notify-v1 is supported")
    })?;
    tracing::info!("using native Wayland idle notifier");

    if !trance_runner::cell_renderer::font_available() {
        return Err(anyhow!(
            "no monospace font found; install fonts-dejavu-core before running trance"
        ));
    }
    if let Some(path) = trance_runner::cell_renderer::resolve_font_path() {
        tracing::info!("using monospace font: {path}");
    }

    let overlay_presenter = OverlayPresenter::new().map(Arc::new).ok_or_else(|| {
        anyhow!("Wayland layer-shell presenter is unavailable on this compositor")
    })?;
    tracing::info!("using Wayland layer-shell presenter");
    Ok((idle_monitor, overlay_presenter))
}

pub fn check_runtime_alive(
    idle_monitor: &IdleMonitor,
    overlay_presenter: &OverlayPresenter,
) -> anyhow::Result<()> {
    if !idle_monitor.is_alive() {
        return Err(anyhow!("Wayland idle monitor connection lost"));
    }
    if !overlay_presenter.is_alive() {
        return Err(anyhow!("Wayland presenter connection lost"));
    }
    Ok(())
}
