// SPDX-License-Identifier: MIT

//! Background idle daemon: Wayland idle detection, overlay presentation, D-Bus API.
//!
//! `run_daemon` is the orchestrator; setup helpers stay here and the runtime
//! tick loop lives in sibling modules.

mod idle_decision;
mod idle_logic;
mod presentation;
mod runtime;
mod tick_loop;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use anyhow::{Context, anyhow};

use crate::config::DaemonConfig;
use crate::controller::DaemonController;

pub use tick_loop::tick_loop_until_shutdown;

#[tracing::instrument(skip_all)]
pub fn run_daemon() -> anyhow::Result<()> {
    check_wayland_env()?;
    let Some(pidfile) = acquire_pidfile()? else {
        return Ok(());
    };
    let controller = Arc::new(DaemonController::new(DaemonConfig::load()));
    crate::config_watcher::start_config_watcher(controller.clone());
    install_signal_handlers(&controller)?;
    log_daemon_startup();
    let dbus_handle = spawn_dbus_thread(Arc::clone(&controller))?;
    let result = tick_loop_until_shutdown(Arc::clone(&controller));
    controller.shutdown.store(true, Ordering::Relaxed);
    let _ = dbus_handle.join();
    release_pidfile(&pidfile);
    result
}

fn check_wayland_env() -> anyhow::Result<()> {
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        return Err(anyhow!(
            "WAYLAND_DISPLAY is not set; trance requires a Wayland session"
        ));
    }
    Ok(())
}

fn pid_file_path() -> std::path::PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("idle-daemon.pid")
    } else {
        std::env::temp_dir().join("idle-daemon.pid")
    }
}

/// Acquire the daemon pid file.
///
/// Returns `Ok(Some(path))` when this process owns the pid file and should
/// release it later. Returns `Ok(None)` when another daemon is already running
/// — the caller should exit cleanly without further setup.
fn acquire_pidfile() -> anyhow::Result<Option<std::path::PathBuf>> {
    let path = pid_file_path();
    if let Some(pid) = fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
    {
        // SAFETY: signal 0 only probes existence; no signal is delivered.
        unsafe {
            if libc::kill(pid, 0) == 0 && pid != std::process::id() as i32 {
                tracing::warn!("idle-daemon is already running (pid {pid}). Exiting.");
                return Ok(None);
            }
        }
    }
    fs::write(&path, std::process::id().to_string())
        .with_context(|| format!("writing pid file to {}", path.display()))?;
    Ok(Some(path))
}

fn release_pidfile(path: &std::path::Path) {
    let _ = fs::remove_file(path);
}

fn install_signal_handlers(controller: &Arc<DaemonController>) -> anyhow::Result<()> {
    signal_hook::flag::register(
        signal_hook::consts::SIGINT,
        Arc::clone(&controller.shutdown),
    )
    .context("registering SIGINT handler")?;
    signal_hook::flag::register(
        signal_hook::consts::SIGTERM,
        Arc::clone(&controller.shutdown),
    )
    .context("registering SIGTERM handler")?;
    Ok(())
}

fn log_daemon_startup() {
    tracing::info!("idle-daemon running (pid {})...", std::process::id());
    if cfg!(debug_assertions) {
        tracing::warn!(
            "WARNING — debug build is very slow (~1 FPS). \
             Use target/release/idle-daemon for real performance."
        );
    }
}

fn spawn_dbus_thread(
    controller: Arc<DaemonController>,
) -> anyhow::Result<std::thread::JoinHandle<()>> {
    let handle = std::thread::spawn(move || {
        if let Err(error) = crate::dbus_server::run(controller) {
            tracing::error!("D-Bus server stopped: {error}");
        }
    });
    Ok(handle)
}
