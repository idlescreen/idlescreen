// SPDX-License-Identifier: MIT

use crate::config::DaemonConfig;
use crate::controller::{DaemonCommand, DaemonController, MAIN_LOOP_INTERVAL};
use crate::presentation::{PluginPresentation, PresentationOptions};
use std::fs;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use trance_runner::launcher::ALLOWED_SAVERS;
use wayland_idle::IdleMonitor;
use wayland_present::OverlayPresenter;

enum ActivePresentation {
    None,
    Plugin(PluginPresentation),
}

impl ActivePresentation {
    fn is_active(&self) -> bool {
        matches!(self, Self::Plugin(_))
    }
}

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        return Err(
            "WAYLAND_DISPLAY is not set; trance requires a Wayland session".into(),
        );
    }

    let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    };

    if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                unsafe {
                    if libc::kill(pid, 0) == 0 && pid != std::process::id() as i32 {
                        eprintln!("trance-daemon is already running (pid {pid}). Exiting.");
                        return Ok(());
                    }
                }
            }
        }
    }

    fs::write(&pid_path, std::process::id().to_string())?;

    let initial_config = DaemonConfig::load();
    let controller = Arc::new(DaemonController::new(initial_config));

    signal_hook::flag::register(
        signal_hook::consts::SIGINT,
        Arc::clone(&controller.shutdown),
    )?;
    signal_hook::flag::register(
        signal_hook::consts::SIGTERM,
        Arc::clone(&controller.shutdown),
    )?;

    println!("trance-daemon running (pid {})...", std::process::id());
    if cfg!(debug_assertions) {
        eprintln!(
            "trance-daemon: WARNING — debug build is very slow (~1 FPS). \
             Use target/release/trance-daemon for real performance."
        );
    }

    let dbus_controller = Arc::clone(&controller);
    let dbus_thread = std::thread::spawn(move || {
        if let Err(error) = crate::dbus_server::run(dbus_controller) {
            eprintln!("trance-daemon: D-Bus server stopped: {error}");
        }
    });

    let idle_timeout = controller.config.lock().unwrap().idle_timeout_mins;
    let idle_monitor = IdleMonitor::new(idle_timeout).ok_or(
        "Wayland idle monitoring is unavailable; ensure ext-idle-notify-v1 is supported",
    )?;
    println!("trance-daemon using native Wayland idle notifier");

    if !trance_runner::cell_renderer::font_available() {
        return Err(
            "no monospace font found; install fonts-dejavu-core before running trance".into(),
        );
    }
    if let Some(path) = trance_runner::cell_renderer::resolve_font_path() {
        println!("trance-daemon using monospace font: {path}");
    }

    let overlay_presenter = OverlayPresenter::new()
        .map(Arc::new)
        .ok_or("Wayland layer-shell presenter is unavailable on this compositor")?;
    println!("trance-daemon using Wayland layer-shell presenter");

    let mut presentation = ActivePresentation::None;
    let mut preview_name: Option<String> = None;
    let mut current_saver = String::new();
    let mut tick_counter = 0u32;

    while !controller.shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(MAIN_LOOP_INTERVAL);
        tick_counter = tick_counter.saturating_add(1);

        for command in controller.drain_commands() {
            match command {
                DaemonCommand::Preview(name) => {
                    preview_name = Some(name);
                }
                DaemonCommand::StopPresentation => {
                    preview_name = None;
                    stop_presentation(Some(&overlay_presenter), &mut presentation);
                    current_saver.clear();
                }
                DaemonCommand::Enable
                | DaemonCommand::Disable
                | DaemonCommand::SetTimeout(_)
                | DaemonCommand::SetSaver(_)
                | DaemonCommand::SetGpuEnabled(_)
                | DaemonCommand::SetShowFpsOverlay(_)
                | DaemonCommand::SetDisplayMode(_)
                | DaemonCommand::SetRenderScale(_) => {
                    let _ = controller.apply_command(command);
                }
            }
        }

        if let Some(timeout) = controller.reload_config_if_due(tick_counter) {
            idle_monitor.set_timeout(timeout);
        }

        let config = controller.config.lock().unwrap().clone();
        let system_idle = idle_monitor.is_idle();
        let session_locked = controller.session_locked.load(Ordering::Relaxed);
        let inhibited = controller.inhibitors.is_inhibited();

        if presentation.is_active() && !overlay_presenter.is_visible() {
            stop_presentation(Some(&overlay_presenter), &mut presentation);
            current_saver.clear();
            preview_name = None;
        }

        if session_locked || inhibited {
            if presentation.is_active() {
                stop_presentation(Some(&overlay_presenter), &mut presentation);
                current_saver.clear();
            }
            preview_name = None;
        } else if let Some(name) = preview_name.clone() {
            if !presentation.is_active() {
                start_presentation(
                    &overlay_presenter,
                    &mut presentation,
                    &mut current_saver,
                    name,
                    "preview",
                    &config,
                );
            }
        } else if config.idle_enabled && system_idle && !presentation.is_active() {
            let seed_micros = current_time_micros();
            let saver_name = pick_saver_name(&config, seed_micros);
            start_presentation(
                &overlay_presenter,
                &mut presentation,
                &mut current_saver,
                saver_name,
                "idle",
                &config,
            );
        } else if presentation.is_active() && !system_idle && preview_name.is_none() {
            stop_presentation(Some(&overlay_presenter), &mut presentation);
            current_saver.clear();
            println!("system activity detected. presentation stopped.");
        } else if !config.idle_enabled && presentation.is_active() {
            stop_presentation(Some(&overlay_presenter), &mut presentation);
            current_saver.clear();
        }

        controller.update_live_state(
            system_idle,
            presentation.is_active(),
            preview_name.is_some(),
            &current_saver,
        );
        controller.publish_status_if_dirty();
    }

    controller.shutdown.store(true, Ordering::Relaxed);
    stop_presentation(Some(&overlay_presenter), &mut presentation);
    let _ = fs::remove_file(pid_path);
    let _ = dbus_thread.join();
    println!("daemon shutdown complete.");
    Ok(())
}

fn start_presentation(
    overlay_presenter: &Arc<OverlayPresenter>,
    presentation: &mut ActivePresentation,
    current_saver: &mut String,
    saver_name: String,
    reason: &str,
    config: &DaemonConfig,
) {
    println!("starting Wayland screensaver '{saver_name}' ({reason})...");
    let launch_mode = if reason == "preview" {
        trance_runner::launcher::LaunchMode::Preview
    } else {
        trance_runner::launcher::LaunchMode::Daemon
    };
    let options = PresentationOptions {
        gpu_enabled: config.gpu_enabled,
        show_fps_overlay: config.show_fps_overlay,
        display_mode: trance_runner::plugin_meta::parse_display_mode(&config.display_mode)
            .unwrap_or(trance_runner::plugin_meta::DisplayMode::PrimaryOnly),
        render_scale: config.render_scale,
        launch_mode,
    };
    match PluginPresentation::start(overlay_presenter.clone(), saver_name.clone(), options) {
        Ok(plugin) => {
            *current_saver = saver_name;
            *presentation = ActivePresentation::Plugin(plugin);
        }
        Err(error) => eprintln!("trance-daemon: failed to start screensaver: {error}"),
    }
}

fn current_time_micros() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_micros() as u64,
        Err(_) => 0,
    }
}

fn pick_saver_name(config: &DaemonConfig, seed_micros: u64) -> String {
    if let Some(ref active) = config.active_saver {
        return active.clone();
    }

    let mut seed = seed_micros;
    seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let index = (seed % ALLOWED_SAVERS.len() as u64) as usize;
    ALLOWED_SAVERS[index].to_string()
}

fn stop_presentation(
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