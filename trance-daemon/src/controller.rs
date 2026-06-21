// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use trance_dbus::DaemonStatus;
use trance_runner::launcher::{resolve_saver_binary, sanitize_saver_name, LaunchMode};

use crate::config::DaemonConfig;
use crate::inhibit::InhibitorState;

#[derive(Debug, Clone)]
pub enum DaemonCommand {
    Enable,
    Disable,
    SetTimeout(u32),
    SetSaver(Option<String>),
    SetGpuEnabled(bool),
    SetShowFpsOverlay(bool),
    SetDisplayMode(String),
    SetRenderScale(Option<f32>),
    Preview(String),
    StopPresentation,
}

pub struct DaemonController {
    pub config: Arc<Mutex<DaemonConfig>>,
    pub status: Arc<Mutex<DaemonStatus>>,
    pub command_tx: mpsc::Sender<DaemonCommand>,
    pub command_rx: Mutex<mpsc::Receiver<DaemonCommand>>,
    pub inhibitors: Arc<InhibitorState>,
    pub session_locked: Arc<AtomicBool>,
    pub shutdown: Arc<AtomicBool>,
    pub status_dirty: Arc<AtomicBool>,
    pub status_emit_tx: Mutex<Option<mpsc::Sender<DaemonStatus>>>,
}

impl DaemonController {
    pub fn new(initial_config: DaemonConfig) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let status = DaemonStatus {
            running: true,
            idle_enabled: initial_config.idle_enabled,
            idle_timeout_mins: initial_config.idle_timeout_mins,
            active_saver: initial_config
                .active_saver
                .clone()
                .unwrap_or_default(),
            gpu_enabled: initial_config.gpu_enabled,
            show_fps_overlay: initial_config.show_fps_overlay,
            display_mode: initial_config.display_mode.clone(),
            render_scale: initial_config
                .render_scale
                .map(|s| s.to_string())
                .unwrap_or_default(),
            ..DaemonStatus::default()
        };

        Self {
            config: Arc::new(Mutex::new(initial_config)),
            status: Arc::new(Mutex::new(status)),
            command_tx,
            command_rx: Mutex::new(command_rx),
            inhibitors: Arc::new(InhibitorState::new()),
            session_locked: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(AtomicBool::new(false)),
            status_dirty: Arc::new(AtomicBool::new(true)),
            status_emit_tx: Mutex::new(None),
        }
    }

    pub fn publish_status_if_dirty(&self) {
        if !self.take_dirty() {
            return;
        }
        let status = self.status.lock().unwrap().clone();
        if let Some(sender) = self.status_emit_tx.lock().unwrap().as_ref() {
            let _ = sender.send(status);
        }
    }

    pub fn drain_commands(&self) -> Vec<DaemonCommand> {
        let mut commands = Vec::new();
        let receiver = self.command_rx.lock().unwrap();
        while let Ok(command) = receiver.try_recv() {
            commands.push(command);
        }
        commands
    }

    pub fn apply_command(&self, command: DaemonCommand) -> Result<(), String> {
        match command {
            DaemonCommand::Enable => {
                let mut config = self.config.lock().unwrap();
                config.idle_enabled = true;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::Disable => {
                let mut config = self.config.lock().unwrap();
                config.idle_enabled = false;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetTimeout(minutes) => {
                if minutes == 0 || minutes > 240 {
                    return Err("timeout must be between 1 and 240 minutes".into());
                }
                let mut config = self.config.lock().unwrap();
                config.idle_timeout_mins = minutes;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetSaver(name) => {
                if let Some(ref saver) = name {
                    sanitize_saver_name(saver)
                        .ok_or_else(|| format!("unknown or invalid screensaver name: {saver}"))?;
                    resolve_saver_binary(saver, &LaunchMode::Daemon)
                        .map_err(|error| error.to_string())?;
                }
                let mut config = self.config.lock().unwrap();
                config.active_saver = name;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetGpuEnabled(enabled) => {
                let mut config = self.config.lock().unwrap();
                config.gpu_enabled = enabled;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetShowFpsOverlay(enabled) => {
                let mut config = self.config.lock().unwrap();
                config.show_fps_overlay = enabled;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetDisplayMode(mode) => {
                if trance_runner::plugin_meta::parse_display_mode(&mode).is_none() {
                    return Err("display_mode must be primary, mirror, or expand".into());
                }
                let mut config = self.config.lock().unwrap();
                config.display_mode = mode;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetRenderScale(scale) => {
                let stored = match scale {
                    None => None,
                    Some(value) if value <= 0.0 => None,
                    Some(value) => {
                        if !(0.25..=1.0).contains(&value) {
                            return Err("render_scale must be between 0.25 and 1.0".into());
                        }
                        Some(value)
                    }
                };
                let mut config = self.config.lock().unwrap();
                config.render_scale = stored;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::Preview(_) | DaemonCommand::StopPresentation => Ok(()),
        }
    }

    pub fn update_live_state(
        &self,
        system_idle: bool,
        presentation_active: bool,
        preview_active: bool,
        current_saver: &str,
    ) {
        let config = self.config.lock().unwrap().clone();
        let session_locked = self.session_locked.load(Ordering::Relaxed);
        let inhibited = self.inhibitors.is_inhibited();

        let mut status = self.status.lock().unwrap();
        let changed = status.system_idle != system_idle
            || status.presentation_active != presentation_active
            || status.preview_active != preview_active
            || status.session_locked != session_locked
            || status.inhibited != inhibited
            || status.idle_enabled != config.idle_enabled
            || status.idle_timeout_mins != config.idle_timeout_mins
            || status.active_saver != config.active_saver.clone().unwrap_or_default()
            || status.gpu_enabled != config.gpu_enabled
            || status.show_fps_overlay != config.show_fps_overlay
            || status.display_mode != config.display_mode
            || status.render_scale
                != config
                    .render_scale
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            || status.current_saver != current_saver;

        status.running = true;
        status.system_idle = system_idle;
        status.presentation_active = presentation_active;
        status.preview_active = preview_active;
        status.session_locked = session_locked;
        status.inhibited = inhibited;
        status.idle_enabled = config.idle_enabled;
        status.idle_timeout_mins = config.idle_timeout_mins;
        status.active_saver = config.active_saver.clone().unwrap_or_default();
        status.gpu_enabled = config.gpu_enabled;
        status.show_fps_overlay = config.show_fps_overlay;
        status.display_mode = config.display_mode.clone();
        status.render_scale = config
            .render_scale
            .map(|s| s.to_string())
            .unwrap_or_default();
        status.current_saver = current_saver.to_string();

        if changed {
            self.status_dirty.store(true, Ordering::Relaxed);
        }
    }

    pub fn mark_dirty(&self) {
        self.status_dirty.store(true, Ordering::Relaxed);
    }

    pub fn take_dirty(&self) -> bool {
        self.status_dirty
            .swap(false, Ordering::Relaxed)
    }

    pub fn reload_config_if_due(&self, tick_counter: u32) -> Option<u32> {
        if tick_counter % 10 != 0 {
            return None;
        }
        let reloaded = DaemonConfig::load();
        let mut config = self.config.lock().unwrap();
        if *config != reloaded {
            *config = reloaded;
            self.mark_dirty();
        }
        Some(config.idle_timeout_mins)
    }
}

pub fn installed_savers() -> Vec<String> {
    trance_runner::discovery::detect_screensavers()
        .into_iter()
        .filter(|name| resolve_saver_binary(name, &LaunchMode::Daemon).is_ok())
        .collect()
}

pub const MAIN_LOOP_INTERVAL: Duration = Duration::from_millis(250);