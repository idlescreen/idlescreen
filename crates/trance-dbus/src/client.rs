// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use zbus::zvariant::OwnedValue;

use crate::status::DaemonStatus;
use crate::{OBJECT_PATH, SERVICE_NAME};

#[zbus::proxy(
    interface = "com.local76.Trance",
    default_service = "com.local76.Trance",
    default_path = "/com/local76/Trance",
    gen_blocking = true
)]
trait Trance {
    fn get_status(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    fn enable(&self) -> zbus::Result<()>;

    fn disable(&self) -> zbus::Result<()>;

    fn set_timeout(&self, minutes: u32) -> zbus::Result<()>;

    fn set_saver(&self, name: &str) -> zbus::Result<()>;

    fn list_savers(&self) -> zbus::Result<Vec<String>>;

    fn preview(&self, name: &str) -> zbus::Result<()>;

    fn stop_preview(&self) -> zbus::Result<()>;

    fn inhibit(&self, application: &str, reason: &str) -> zbus::Result<u32>;

    fn un_inhibit(&self, cookie: u32) -> zbus::Result<()>;

    fn set_gpu_enabled(&self, enabled: bool) -> zbus::Result<()>;

    fn set_show_fps_overlay(&self, enabled: bool) -> zbus::Result<()>;

    fn set_display_mode(&self, mode: &str) -> zbus::Result<()>;

    fn set_render_scale(&self, scale: f64) -> zbus::Result<()>;
}

/// Blocking D-Bus client for the trance daemon.
pub struct TranceClient {
    connection: zbus::blocking::Connection,
}

impl TranceClient {
    pub fn connect() -> zbus::Result<Self> {
        let connection = zbus::blocking::Connection::session()?;
        Ok(Self { connection })
    }

    pub fn get_status(&self) -> zbus::Result<DaemonStatus> {
        parse_status(self.proxy()?.get_status()?)
    }

    pub fn enable(&self) -> zbus::Result<()> {
        self.proxy()?.enable()
    }

    pub fn disable(&self) -> zbus::Result<()> {
        self.proxy()?.disable()
    }

    pub fn set_timeout(&self, minutes: u32) -> zbus::Result<()> {
        self.proxy()?.set_timeout(minutes)
    }

    pub fn set_saver(&self, name: &str) -> zbus::Result<()> {
        self.proxy()?.set_saver(name)
    }

    pub fn list_savers(&self) -> zbus::Result<Vec<String>> {
        self.proxy()?.list_savers()
    }

    pub fn preview(&self, name: &str) -> zbus::Result<()> {
        self.proxy()?.preview(name)
    }

    pub fn stop_preview(&self) -> zbus::Result<()> {
        self.proxy()?.stop_preview()
    }

    pub fn inhibit(&self, application: &str, reason: &str) -> zbus::Result<u32> {
        self.proxy()?.inhibit(application, reason)
    }

    pub fn un_inhibit(&self, cookie: u32) -> zbus::Result<()> {
        self.proxy()?.un_inhibit(cookie)
    }

    pub fn set_gpu_enabled(&self, enabled: bool) -> zbus::Result<()> {
        self.proxy()?.set_gpu_enabled(enabled)
    }

    pub fn set_show_fps_overlay(&self, enabled: bool) -> zbus::Result<()> {
        self.proxy()?.set_show_fps_overlay(enabled)
    }

    pub fn set_display_mode(&self, mode: &str) -> zbus::Result<()> {
        self.proxy()?.set_display_mode(mode)
    }

    pub fn set_render_scale(&self, scale: f32) -> zbus::Result<()> {
        self.proxy()?.set_render_scale(f64::from(scale))
    }

    fn proxy(&self) -> zbus::Result<TranceProxyBlocking<'_>> {
        TranceProxyBlocking::new(&self.connection)
    }
}

fn parse_status(map: HashMap<String, OwnedValue>) -> zbus::Result<DaemonStatus> {
    Ok(DaemonStatus {
        running: read_bool(&map, "running"),
        idle_enabled: read_bool(&map, "idle_enabled"),
        idle_timeout_mins: read_u32(&map, "idle_timeout_mins"),
        active_saver: read_string(&map, "active_saver"),
        presentation_active: read_bool(&map, "presentation_active"),
        preview_active: read_bool(&map, "preview_active"),
        system_idle: read_bool(&map, "system_idle"),
        session_locked: read_bool(&map, "session_locked"),
        inhibited: read_bool(&map, "inhibited"),
        current_saver: read_string(&map, "current_saver"),
        gpu_enabled: read_bool(&map, "gpu_enabled"),
        show_fps_overlay: read_bool(&map, "show_fps_overlay"),
        display_mode: read_string(&map, "display_mode"),
        render_scale: read_string(&map, "render_scale"),
    })
}

fn read_bool(map: &HashMap<String, OwnedValue>, key: &str) -> bool {
    if let Some(value) = map.get(key) {
        if let Ok(flag) = value.downcast_ref::<bool>() {
            return flag;
        }
    }
    false
}

fn read_u32(map: &HashMap<String, OwnedValue>, key: &str) -> u32 {
    if let Some(value) = map.get(key) {
        if let Ok(number) = value.downcast_ref::<u32>() {
            return number;
        }
    }
    0
}

fn read_string(map: &HashMap<String, OwnedValue>, key: &str) -> String {
    if let Some(value) = map.get(key) {
        if let Ok(text) = value.downcast_ref::<String>() {
            return text.clone();
        }
    }
    String::new()
}

/// Returns whether the trance daemon is reachable on the session bus.
pub fn daemon_available() -> bool {
    let connection = match zbus::blocking::Connection::session() {
        Ok(connection) => connection,
        Err(_) => return false,
    };
    connection
        .call_method(
            Some(SERVICE_NAME),
            OBJECT_PATH,
            Some("org.freedesktop.DBus.Peer"),
            "Ping",
            &(),
        )
        .is_ok()
}