// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! D-Bus service interfaces for org.freedesktop.ScreenSaver and io.github.idlescreen.Idle.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use zbus::interface;

#[derive(Clone)]
pub struct InhibitEntry {
    pub cookie: u32,
    pub app_name: String,
    pub reason: String,
}

pub struct ScreenSaverState {
    pub active: AtomicBool,
    pub started_at: Mutex<Option<Instant>>,
    pub inhibitors: Mutex<Vec<InhibitEntry>>,
    pub cookie_counter: AtomicU32,
    pub current_saver: Mutex<String>,
    pub text_targets: Mutex<Vec<String>>,
    pub custom_text: Mutex<String>,
    pub custom_text_epoch: std::sync::atomic::AtomicU64,
    pub wake_handle: Mutex<Option<crate::daemon::gnome::WakeHandle>>,
}

impl Default for ScreenSaverState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenSaverState {
    pub fn new() -> Self {
        let cfg = crate::daemon::config::Config::load_or_migrate();
        let mut text_targets = cfg.saver.ascii.targets;
        if text_targets.is_empty() {
            text_targets = crate::daemon::config::parse_targets_from_mode(&cfg.saver.ascii.mode);
        }
        Self {
            active: AtomicBool::new(false),
            started_at: Mutex::new(None),
            inhibitors: Mutex::new(Vec::new()),
            cookie_counter: AtomicU32::new(1),
            current_saver: Mutex::new(cfg.saver.active.unwrap_or_else(|| "ascii".to_string())),
            text_targets: Mutex::new(text_targets),
            custom_text: Mutex::new(cfg.saver.ascii.text),
            custom_text_epoch: std::sync::atomic::AtomicU64::new(1),
            wake_handle: Mutex::new(None),
        }
    }

    pub fn set_wake_handle(&self, handle: crate::daemon::gnome::WakeHandle) {
        if let Ok(mut guard) = self.wake_handle.lock() {
            *guard = Some(handle);
        }
    }

    pub fn clear_wake_handle(&self) {
        if let Ok(mut guard) = self.wake_handle.lock() {
            *guard = None;
        }
    }

    pub fn wake(&self) {
        if let Ok(guard) = self.wake_handle.lock() {
            if let Some(ref h) = *guard {
                h.wake();
            }
        }
    }

    pub fn is_inhibited(&self) -> bool {
        if let Ok(guard) = self.inhibitors.lock() {
            !guard.is_empty()
        } else {
            false
        }
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
        if let Ok(mut start) = self.started_at.lock() {
            *start = None;
        }
        self.wake();
    }

    pub fn set_saver(&self, name: String) {
        if let Ok(mut saver) = self.current_saver.lock() {
            *saver = name;
        }
        self.wake();
    }

    pub fn get_text_targets(&self) -> Vec<String> {
        self.text_targets.lock().map(|t| t.clone()).unwrap_or_default()
    }

    pub fn set_text_targets(&self, targets: &[String]) {
        if let Ok(mut t) = self.text_targets.lock() {
            *t = targets.to_vec();
        }
        let cfg_dir = crate::daemon::config::resolve_config_dir();
        let toml_path = cfg_dir.join("config.toml");
        let mut config = crate::daemon::config::Config::load_or_migrate();
        config.saver.ascii.targets = targets.to_vec();
        config.saver.ascii.mode = targets.join(",");
        if let Ok(s) = toml::to_string_pretty(&config) {
            let _ = std::fs::create_dir_all(&cfg_dir);
            let _ = std::fs::write(&toml_path, s);
        }
        self.wake();
    }

    pub fn set_text(&self, text: &str) {
        if let Ok(mut t) = self.custom_text.lock() {
            *t = text.to_string();
            self.custom_text_epoch.fetch_add(1, Ordering::Release);
        }
        let cfg_dir = crate::daemon::config::resolve_config_dir();
        let toml_path = cfg_dir.join("config.toml");
        let mut config = crate::daemon::config::Config::load_or_migrate();
        config.saver.ascii.text = text.to_string();
        if let Ok(s) = toml::to_string_pretty(&config) {
            let _ = std::fs::create_dir_all(&cfg_dir);
            let _ = std::fs::write(&toml_path, s);
        }
        self.wake();
    }
}

pub struct ScreenSaverInterface {
    pub state: Arc<ScreenSaverState>,
}

#[interface(name = "org.freedesktop.ScreenSaver")]
impl ScreenSaverInterface {
    fn lock(&self) {
        self.state.active.store(true, Ordering::SeqCst);
        if let Ok(mut start) = self.state.started_at.lock() {
            *start = Some(Instant::now());
        }
        self.state.wake();
    }

    fn simulate_user_activity(&self) {
        self.state.active.store(false, Ordering::SeqCst);
        if let Ok(mut start) = self.state.started_at.lock() {
            *start = None;
        }
        self.state.wake();
    }

    fn get_active(&self) -> bool {
        self.state.active.load(Ordering::SeqCst)
    }

    fn get_active_time(&self) -> u32 {
        if let Ok(guard) = self.state.started_at.lock()
            && let Some(start) = *guard
        {
            return start.elapsed().as_secs() as u32;
        }
        0
    }

    fn inhibit(&self, application_name: String, reason_for_inhibit: String) -> u32 {
        let cookie = self.state.cookie_counter.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut guard) = self.state.inhibitors.lock() {
            guard.push(InhibitEntry {
                cookie,
                app_name: application_name,
                reason: reason_for_inhibit,
            });
        }
        self.state.wake();
        cookie
    }

    fn un_inhibit(&self, cookie: u32) {
        if let Ok(mut guard) = self.state.inhibitors.lock() {
            guard.retain(|e| e.cookie != cookie);
        }
        self.state.wake();
    }

    fn throttle(&self, application_name: String, reason_for_inhibit: String) -> u32 {
        self.inhibit(application_name, reason_for_inhibit)
    }

    fn un_throttle(&self, cookie: u32) {
        self.un_inhibit(cookie);
    }
}

pub struct IdleInterface {
    pub state: Arc<ScreenSaverState>,
}

#[interface(name = "io.github.idlescreen.Idle")]
impl IdleInterface {
    fn get_status(&self) -> String {
        let active = self.state.active.load(Ordering::SeqCst);
        let saver = self
            .state
            .current_saver
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let inhibited = self.state.is_inhibited();
        format!("{{\"active\":{active},\"saver\":\"{saver}\",\"inhibited\":{inhibited}}}")
    }

    fn stop(&self) {
        self.state.active.store(false, Ordering::SeqCst);
        if let Ok(mut start) = self.state.started_at.lock() {
            *start = None;
        }
    }

    fn set_saver(&self, name: String) {
        if let Ok(mut saver) = self.state.current_saver.lock() {
            *saver = name;
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn set_text_targets(&self, targets: Vec<String>) {
        self.state.set_text_targets(&targets);
    }

    fn get_text_targets(&self) -> Vec<String> {
        self.state.get_text_targets()
    }

    #[allow(clippy::needless_pass_by_value)]
    fn set_text(&self, text: String) {
        self.state.set_text(&text);
    }

    fn list_savers(&self) -> Vec<String> {
        let mut list = vec!["ascii".to_string()];
        if crate::extras_ipc::ExtrasSession::is_available() {
            for &s in &[
                "aurora", "beams", "bursts", "chaos", "cosmos", "glyphs", "gnats", "hearth",
                "radar", "ripple", "storm",
            ] {
                list.push(s.to_string());
            }
        }
        list
    }
}

#[zbus::proxy(
    interface = "io.github.idlescreen.Idle",
    default_service = "io.github.idlescreen.Idle",
    default_path = "/io/github/idlescreen/Idle"
)]
pub trait IdleClient {
    fn get_status(&self) -> zbus::Result<String>;
    fn stop(&self) -> zbus::Result<()>;
    fn set_saver(&self, name: &str) -> zbus::Result<()>;
    fn set_text_targets(&self, targets: &[String]) -> zbus::Result<()>;
    fn get_text_targets(&self) -> zbus::Result<Vec<String>>;
    fn set_text(&self, text: &str) -> zbus::Result<()>;
    fn list_savers(&self) -> zbus::Result<Vec<String>>;
}
