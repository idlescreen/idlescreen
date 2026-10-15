// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared thread-safe daemon state across D-Bus, control socket, and idle/session watchers.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use super::config::{Config, parse_targets_from_mode};
pub use super::dbus::{InhibitEntry, VALID_SAVERS, canonical_saver_name, is_valid_saver};
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
        let text_targets = if cfg.saver.ascii.targets.is_empty() {
            parse_targets_from_mode(&cfg.saver.ascii.mode)
        } else {
            cfg.saver.ascii.targets
        };
        let active_saver = canonical_saver_name(cfg.saver.active.as_deref().unwrap_or("ascii"));
        Self {
            active: AtomicBool::new(false),
            idle_enabled: AtomicBool::new(cfg.general.idle_enabled),
            idle_timeout_secs: AtomicU64::new(cfg.general.idle_timeout_secs),
            started_at: Mutex::new(None),
            inhibitors: Mutex::new(Vec::new()),
            cookie_counter: AtomicU32::new(1),
            current_saver: Mutex::new(active_saver),
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

    pub fn reload_from_disk(&self) {
        let cfg = Config::load_or_migrate();
        self.idle_enabled
            .store(cfg.general.idle_enabled, Ordering::SeqCst);
        self.idle_timeout_secs
            .store(cfg.general.idle_timeout_secs, Ordering::SeqCst);
        if let Some(active) = cfg.saver.active
            && let Ok(mut saver) = self.current_saver.lock()
        {
            *saver = canonical_saver_name(&active);
        }
        if let Ok(mut t) = self.text_targets.lock() {
            *t = if cfg.saver.ascii.targets.is_empty() {
                parse_targets_from_mode(&cfg.saver.ascii.mode)
            } else {
                cfg.saver.ascii.targets
            };
        }
        if let Ok(mut c) = self.custom_text.lock() {
            *c = cfg.saver.ascii.text;
        }
        self.custom_text_epoch.fetch_add(1, Ordering::Release);
        self.wake_idle_cfg();
        self.wake();
    }

    pub fn set_saver(&self, name: &str) -> bool {
        let raw = name.trim();
        if !is_valid_saver(raw) {
            return false;
        }
        let trimmed = canonical_saver_name(raw);
        if let Ok(mut saver) = self.current_saver.lock() {
            (*saver).clone_from(&trimmed);
        }
        let mut config = Config::load_or_migrate();
        if config.saver.active.as_deref() != Some(trimmed.as_str()) {
            config.saver.active = Some(trimmed);
            let _ = config.save();
        }
        self.wake();
        true
    }

    pub fn set_timeout(&self, secs: u64) {
        self.idle_timeout_secs.store(secs, Ordering::SeqCst);
        let mut config = Config::load_or_migrate();
        if config.general.idle_timeout_secs != secs {
            config.general.idle_timeout_secs = secs;
            let _ = config.save();
        }
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
        let mode = targets.join(",");
        if config.saver.ascii.targets != targets || config.saver.ascii.mode != mode {
            config.saver.ascii.targets = targets.to_vec();
            config.saver.ascii.mode = mode;
            let _ = config.save();
        }
        self.wake();
    }

    pub fn set_text(&self, text: &str) {
        if let Ok(mut t) = self.custom_text.lock() {
            *t = text.to_string();
            self.custom_text_epoch.fetch_add(1, Ordering::Release);
        }
        let mut config = Config::load_or_migrate();
        if config.saver.ascii.text != text {
            config.saver.ascii.text = text.to_string();
            let _ = config.save();
        }
        self.wake();
    }

    pub fn dispatch_control_command(&self, cmd: &str) -> (String, bool) {
        let out = match cmd {
            "status" => self.format_status_json(),
            "inhibitors" => {
                let e = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
                let v: Vec<String> = self
                    .list_inhibitors()
                    .into_iter()
                    .map(|(c, a, r)| {
                        let (a, r) = (e(&a), e(&r));
                        format!("{{\"cookie\":{c},\"app\":\"{a}\",\"reason\":\"{r}\"}}")
                    })
                    .collect();
                format!("[{}]", v.join(","))
            }
            "reload" => {
                self.reload_from_disk();
                "OK: reloaded".to_string()
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
            _ if cmd.starts_with("timeout ") => cmd[8..].trim().parse::<u64>().map_or_else(
                |_| "ERR: invalid timeout".to_string(),
                |secs| {
                    self.set_timeout(secs);
                    format!("OK: timeout set to {secs}s")
                },
            ),
            _ if cmd.starts_with("text_targets ") => {
                self.set_text_targets(&parse_targets_from_mode(&cmd[13..]));
                "OK: text targets updated".to_string()
            }
            _ if cmd.starts_with("set_text ") => {
                self.set_text(cmd[9..].trim());
                "OK: custom text updated".to_string()
            }
            _ => "UNKNOWN_COMMAND".to_string(),
        };
        (out, false)
    }
}
