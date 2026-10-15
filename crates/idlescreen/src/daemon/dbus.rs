// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! D-Bus service interfaces for org.freedesktop.ScreenSaver and io.github.idlescreen.Idle.

pub use super::state::ScreenSaverState;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use zbus::interface;

#[rustfmt::skip]
pub const VALID_SAVERS: &[&str] = &[
    "ascii", "superfedora", "tour", "new", "aurora", "beams", "chaos", "cosmos", "radar",
    "ripple", "plasma", "tunnel", "breakout", "tetris", "pipes", "mystify", "life", "metaballs",
];

pub fn is_valid_saver(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    VALID_SAVERS.contains(&lower.as_str())
        || crate::ascii::effects::EffectKind::parse(&lower).is_some()
}

pub fn canonical_saver_name(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    if lower == "new" {
        "tour".to_string()
    } else if let Some(kind) = crate::ascii::effects::EffectKind::parse(&lower) {
        kind.name().to_string()
    } else {
        lower
    }
}

#[derive(Clone, Debug)]
pub struct InhibitEntry {
    pub cookie: u32,
    pub app_name: String,
    pub reason: String,
    pub sender: Option<String>,
}

#[rustfmt::skip]
impl ScreenSaverState {
    pub fn is_inhibited(&self) -> bool {
        self.inhibitors.lock().is_ok_and(|g| !g.is_empty())
            || (!cfg!(test) && super::watchers::media_inhibit::is_media_or_system_inhibited())
    }

    pub fn format_status_json(&self) -> String {
        let active = self.active.load(Ordering::SeqCst);
        let saver = self.current_saver.lock().map_or_else(|_| "ascii".into(), |s| s.clone());
        let inhibited = self.is_inhibited();
        let timeout = self.idle_timeout_secs.load(Ordering::SeqCst);
        format!("{{\"running\":true,\"active\":{active},\"saver\":\"{saver}\",\"inhibited\":{inhibited},\"timeout\":{timeout}}}")
    }

    pub fn list_inhibitors(&self) -> Vec<(u32, String, String)> {
        let mut list = self.inhibitors.lock().map_or_else(
            |_| Vec::new(),
            |g| g.iter().map(|e| (e.cookie, e.app_name.clone(), e.reason.clone())).collect(),
        );
        if !cfg!(test) {
            list.extend(super::watchers::media_inhibit::probe_active_inhibitors());
        }
        list
    }

    pub fn remove_inhibitors_by_sender(&self, sender: &str) {
        if !sender.is_empty() && let Ok(mut g) = self.inhibitors.lock() {
            let before = g.len();
            g.retain(|e| e.sender.as_deref() != Some(sender));
            if g.len() != before {
                drop(g);
                self.wake_idle_cfg();
                self.wake();
            }
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
        self.state.wake();
    }

    fn simulate_user_activity(&self) {
        self.state.stop();
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

    #[allow(clippy::needless_pass_by_value)]
    fn inhibit(
        &self,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        application_name: String,
        reason_for_inhibit: String,
    ) -> u32 {
        let cookie = self.state.cookie_counter.fetch_add(1, Ordering::SeqCst);
        let sender = hdr.sender().map(std::string::ToString::to_string);
        if let Ok(mut guard) = self.state.inhibitors.lock() {
            guard.push(InhibitEntry {
                cookie,
                app_name: application_name,
                reason: reason_for_inhibit,
                sender,
            });
        }
        if self.state.active.load(Ordering::SeqCst) {
            self.state.stop();
        } else {
            self.state.wake();
        }
        cookie
    }

    fn un_inhibit(&self, cookie: u32) {
        if let Ok(mut guard) = self.state.inhibitors.lock() {
            guard.retain(|e| e.cookie != cookie);
        }
        self.state.wake_idle_cfg();
        self.state.wake();
    }

    #[allow(clippy::needless_pass_by_value)]
    fn throttle(
        &self,
        #[zbus(header)] hdr: zbus::message::Header<'_>,
        application_name: String,
        reason_for_inhibit: String,
    ) -> u32 {
        self.inhibit(hdr, application_name, reason_for_inhibit)
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
        self.state.format_status_json()
    }

    fn stop(&self) {
        self.state.stop();
    }

    #[allow(clippy::needless_pass_by_value)]
    fn set_saver(&self, name: String) -> bool {
        self.state.set_saver(&name)
    }

    fn set_timeout(&self, secs: u64) {
        self.state.set_timeout(secs);
    }

    fn list_inhibitors(&self) -> Vec<(u32, String, String)> {
        self.state.list_inhibitors()
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
        let mut list = vec!["ascii".to_string(), "tour".to_string()];
        if crate::ascii::sys_info::is_superfedora_machine() {
            list.push("superfedora".to_string());
        }
        for kind in crate::ascii::effects::EffectKind::ALL {
            list.push(kind.name().to_string());
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
    fn set_saver(&self, name: &str) -> zbus::Result<bool>;
    fn set_timeout(&self, secs: u64) -> zbus::Result<()>;
    fn list_inhibitors(&self) -> zbus::Result<Vec<(u32, String, String)>>;
    fn set_text_targets(&self, targets: &[String]) -> zbus::Result<()>;
    fn get_text_targets(&self) -> zbus::Result<Vec<String>>;
    fn set_text(&self, text: &str) -> zbus::Result<()>;
    fn list_savers(&self) -> zbus::Result<Vec<String>>;
}

#[rustfmt::skip]
pub fn register_dbus_services(state: Arc<ScreenSaverState>) -> Option<zbus::blocking::Connection> {
    use super::watchers::session::{GnomeClientPrivateInterface, GnomeSessionManagerInterface};
    let builder = zbus::blocking::connection::Builder::session().ok()?;
    let conn = builder
        .name("io.github.idlescreen.Idle").ok()?
        .serve_at("/io/github/idlescreen/Idle", IdleInterface { state: state.clone() }).ok()?
        .serve_at("/org/freedesktop/ScreenSaver", ScreenSaverInterface { state: state.clone() }).ok()?
        .serve_at("/org/gnome/SessionManager", GnomeSessionManagerInterface { state }).ok()?
        .serve_at("/org/gnome/SessionManager/Client1", GnomeClientPrivateInterface).ok()?
        .build().ok()?;
    let _ = conn.request_name("org.freedesktop.ScreenSaver");
    let _ = conn.request_name("org.gnome.SessionManager");
    Some(conn)
}
