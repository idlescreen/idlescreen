// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared thread-safe daemon state across D-Bus, control socket, and idle/session watchers.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use super::config::{Config, parse_targets_from_mode};
pub use super::dbus::{InhibitEntry, VALID_SAVERS, is_valid_saver};
use super::watchers::WakeHandle;

pub struct ScreenSaverState {
    pub active: AtomicBool,
    pub idle_enabled: AtomicBool,
    pub idle_timeout_secs: AtomicU64,
    pub started_at: Mutex<Option<Instant>>,
    pub inhibitors: Mutex<Vec<InhibitEntry>>,
    pub cookie_counter: AtomicU32,
    pub current_saver: Mutex<String>,
    pub text_targets: Mutex<Vec<String>>,
    pub custom_text: Mutex<String>,
    pub custom_text_epoch: AtomicU64,
    pub wake_handle: Mutex<Option<WakeHandle>>,
    pub idle_cfg_wake: Mutex<Option<WakeHandle>>,
}

impl Default for ScreenSaverState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenSaverState {
    pub fn new() -> Self {
        let cfg = Config::load_or_migrate();
        let mut text_targets = cfg.saver.ascii.targets;
        if text_targets.is_empty() {
            text_targets = parse_targets_from_mode(&cfg.saver.ascii.mode);
        }
        Self {
            active: AtomicBool::new(false),
            idle_enabled: AtomicBool::new(cfg.general.idle_enabled),
            idle_timeout_secs: AtomicU64::new(cfg.general.idle_timeout_secs),
            started_at: Mutex::new(None),
            inhibitors: Mutex::new(Vec::new()),
            cookie_counter: AtomicU32::new(1),
            current_saver: Mutex::new(cfg.saver.active.unwrap_or_else(|| "ascii".to_string())),
            text_targets: Mutex::new(text_targets),
            custom_text: Mutex::new(cfg.saver.ascii.text),
            custom_text_epoch: AtomicU64::new(1),
            wake_handle: Mutex::new(None),
            idle_cfg_wake: Mutex::new(None),
        }
    }

    pub fn set_wake_handle(&self, handle: WakeHandle) {
        if let Ok(mut g) = self.wake_handle.lock() {
            *g = Some(handle);
        }
    }

    pub fn set_idle_cfg_wake(&self, handle: WakeHandle) {
        if let Ok(mut g) = self.idle_cfg_wake.lock() {
            *g = Some(handle);
        }
    }

    pub fn clear_wake_handle(&self) {
        if let Ok(mut g) = self.wake_handle.lock() {
            *g = None;
        }
        if let Ok(mut g) = self.idle_cfg_wake.lock() {
            *g = None;
        }
    }

    pub fn wake(&self) {
        if let Ok(g) = self.wake_handle.lock()
            && let Some(ref h) = *g
        {
            h.wake();
        }
    }

    pub fn wake_idle_cfg(&self) {
        if let Ok(g) = self.idle_cfg_wake.lock()
            && let Some(ref h) = *g
        {
            h.wake();
        }
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
        if let Ok(mut start) = self.started_at.lock() {
            *start = None;
        }
        self.wake();
    }

    pub fn set_saver(&self, name: &str) -> bool {
        let trimmed = name.trim().to_string();
        if !is_valid_saver(&trimmed) {
            return false;
        }
        if let Ok(mut saver) = self.current_saver.lock() {
            (*saver).clone_from(&trimmed);
        }
        let mut config = Config::load_or_migrate();
        config.saver.active = Some(trimmed);
        let _ = config.save();
        self.wake();
        true
    }

    pub fn set_timeout(&self, secs: u64) {
        self.idle_timeout_secs.store(secs, Ordering::SeqCst);
        let mut config = Config::load_or_migrate();
        config.general.idle_timeout_secs = secs;
        let _ = config.save();
        self.wake_idle_cfg();
        self.wake();
    }

    pub fn get_text_targets(&self) -> Vec<String> {
        self.text_targets
            .lock()
            .map_or_else(|_| Vec::new(), |t| t.clone())
    }

    pub fn set_text_targets(&self, targets: &[String]) {
        if let Ok(mut t) = self.text_targets.lock() {
            *t = targets.to_vec();
            self.custom_text_epoch.fetch_add(1, Ordering::Release);
        }
        let mut config = Config::load_or_migrate();
        config.saver.ascii.targets = targets.to_vec();
        config.saver.ascii.mode = targets.join(",");
        let _ = config.save();
        self.wake();
    }

    pub fn set_text(&self, text: &str) {
        if let Ok(mut t) = self.custom_text.lock() {
            *t = text.to_string();
            self.custom_text_epoch.fetch_add(1, Ordering::Release);
        }
        let mut config = Config::load_or_migrate();
        config.saver.ascii.text = text.to_string();
        let _ = config.save();
        self.wake();
    }

    pub fn dispatch_control_command(&self, cmd: &str) -> (String, bool) {
        let out = match cmd {
            "status" => {
                let active = self.active.load(Ordering::SeqCst);
                let saver = self
                    .current_saver
                    .lock()
                    .map_or_else(|_| "ascii".into(), |s| s.clone());
                let (inhibited, timeout) = (
                    self.is_inhibited(),
                    self.idle_timeout_secs.load(Ordering::SeqCst),
                );
                format!(
                    "{{\"running\":true,\"active\":{active},\"saver\":\"{saver}\",\"inhibited\":{inhibited},\"timeout\":{timeout}}}"
                )
            }
            "inhibitors" => {
                let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
                let items: Vec<String> = self
                    .list_inhibitors()
                    .into_iter()
                    .map(|(c, a, r)| {
                        format!(
                            "{{\"cookie\":{c},\"app\":\"{}\",\"reason\":\"{}\"}}",
                            esc(&a),
                            esc(&r)
                        )
                    })
                    .collect();
                format!("[{}]", items.join(","))
            }
            "stop" => {
                self.stop();
                "OK".to_string()
            }
            "start" | "lock" => {
                if !self.is_inhibited() {
                    self.active.store(true, Ordering::SeqCst);
                    self.wake();
                }
                "OK".to_string()
            }
            "quit" => {
                self.stop();
                return ("OK: daemon exiting".to_string(), true);
            }
            "ping" | "PING" => "PONG".to_string(),
            _ if cmd.starts_with("set ") => {
                let s = cmd[4..].trim();
                if self.set_saver(s) {
                    format!("OK: saver set to {s}")
                } else {
                    format!("ERR: unknown saver {s}")
                }
            }
            _ if cmd.starts_with("timeout ") => {
                if let Ok(secs) = cmd["timeout ".len()..].trim().parse::<u64>() {
                    self.set_timeout(secs);
                    format!("OK: timeout set to {secs}s")
                } else {
                    "ERR: invalid timeout".to_string()
                }
            }
            _ if cmd.starts_with("text_targets ") => {
                let targets: Vec<String> = cmd["text_targets ".len()..]
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
                self.set_text_targets(&targets);
                "OK: text targets updated".to_string()
            }
            _ if cmd.starts_with("set_text ") => {
                self.set_text(cmd["set_text ".len()..].trim());
                "OK: custom text updated".to_string()
            }
            _ => "UNKNOWN_COMMAND".to_string(),
        };
        (out, false)
    }
}
