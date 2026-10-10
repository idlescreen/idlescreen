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
}

impl Default for ScreenSaverState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenSaverState {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            started_at: Mutex::new(None),
            inhibitors: Mutex::new(Vec::new()),
            cookie_counter: AtomicU32::new(1),
            current_saver: Mutex::new("ascii".to_string()),
        }
    }

    pub fn is_inhibited(&self) -> bool {
        if let Ok(guard) = self.inhibitors.lock() {
            !guard.is_empty()
        } else {
            false
        }
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
    }

    fn simulate_user_activity(&self) {
        self.state.active.store(false, Ordering::SeqCst);
        if let Ok(mut start) = self.state.started_at.lock() {
            *start = None;
        }
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
        cookie
    }

    fn un_inhibit(&self, cookie: u32) {
        if let Ok(mut guard) = self.state.inhibitors.lock() {
            guard.retain(|e| e.cookie != cookie);
        }
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
    fn list_savers(&self) -> zbus::Result<Vec<String>>;
}
