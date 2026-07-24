// SPDX-License-Identifier: MIT

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use zbus::names::UniqueName;

#[derive(Debug, Clone)]
pub struct Inhibitor {
    pub cookie: u32,
    #[allow(dead_code)]
    pub application_name: String,
    #[allow(dead_code)]
    pub reason: String,
    pub client: UniqueName<'static>,
}

#[derive(Debug)]
pub struct InhibitorState {
    inhibitors: Mutex<Vec<Inhibitor>>,
    last_cookie: AtomicU32,
    #[cfg(not(test))]
    logind_cache: Mutex<(bool, std::time::Instant)>,
}

impl InhibitorState {
    pub fn new() -> Self {
        Self {
            inhibitors: Mutex::new(Vec::new()),
            last_cookie: AtomicU32::new(0),
            #[cfg(not(test))]
            logind_cache: Mutex::new((
                false,
                std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(5))
                    .unwrap_or_else(std::time::Instant::now),
            )),
        }
    }

    pub fn is_inhibited(&self) -> bool {
        if !self
            .inhibitors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
        {
            return true;
        }

        #[cfg(test)]
        {
            false
        }
        #[cfg(not(test))]
        {
            let mut cache = self.logind_cache.lock().unwrap_or_else(|e| e.into_inner());
            if cache.1.elapsed() >= std::time::Duration::from_secs(2) {
                cache.0 = check_logind_inhibited() || check_mpris_playing();
                cache.1 = std::time::Instant::now();
            }
            cache.0
        }
    }

    pub fn add(
        &self,
        application_name: String,
        reason: String,
        client: UniqueName<'static>,
    ) -> Result<u32, &'static str> {
        let mut inhibitors = self.inhibitors.lock().unwrap_or_else(|e| e.into_inner());
        let count = inhibitors
            .iter()
            .filter(|entry| entry.client == client)
            .count();
        if count >= 32 {
            return Err("too many concurrent inhibitors for this client");
        }
        let cookie = self.last_cookie.fetch_add(1, Ordering::Relaxed) + 1;
        inhibitors.push(Inhibitor {
            cookie,
            application_name,
            reason,
            client,
        });
        Ok(cookie)
    }

    pub fn add_with_cookie(
        &self,
        application_name: String,
        reason: String,
        client: UniqueName<'static>,
        cookie: u32,
    ) {
        let mut inhibitors = self.inhibitors.lock().unwrap_or_else(|e| e.into_inner());
        if inhibitors
            .iter()
            .any(|entry| entry.cookie == cookie && entry.client == client)
        {
            return;
        }
        tracing::info!(
            "Adding external inhibitor for client {} (cookie {}): {}",
            client,
            cookie,
            application_name
        );
        inhibitors.push(Inhibitor {
            cookie,
            application_name,
            reason,
            client,
        });
    }

    /// Remove an inhibitor only when `cookie` belongs to `client`.
    pub fn remove_for_client(&self, cookie: u32, client: &UniqueName<'_>) -> bool {
        let mut inhibitors = self.inhibitors.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(index) = inhibitors
            .iter()
            .position(|entry| entry.cookie == cookie && entry.client == *client)
        {
            inhibitors.remove(index);
            true
        } else {
            false
        }
    }

    pub fn remove_client(&self, client: &UniqueName<'_>) {
        let mut inhibitors = self.inhibitors.lock().unwrap_or_else(|e| e.into_inner());
        inhibitors.retain(|entry| entry.client != *client);
    }

    pub fn list(&self) -> Vec<(u32, String, String)> {
        let inhibitors = self.inhibitors.lock().unwrap_or_else(|e| e.into_inner());
        inhibitors
            .iter()
            .map(|entry| {
                (
                    entry.cookie,
                    entry.application_name.clone(),
                    entry.reason.clone(),
                )
            })
            .collect()
    }
}

#[cfg(all(target_os = "linux", not(test)))]
type LogindInhibitorInfo = (String, String, String, String, u32, u32);

#[cfg(all(target_os = "linux", not(test)))]
fn check_logind_inhibited() -> bool {
    let run_blocking = || {
        let Ok(conn) = zbus::blocking::Connection::system() else {
            return false;
        };
        let Ok(reply) = conn.call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "ListInhibitors",
            &(),
        ) else {
            return false;
        };
        let Ok(inhibitors): Result<Vec<LogindInhibitorInfo>, _> = reply.body().deserialize() else {
            return false;
        };
        for (what, _, _, _, _, _) in inhibitors {
            if what.split(':').any(|w| w == "idle") {
                return true;
            }
        }
        false
    };

    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(run_blocking)
    } else {
        run_blocking()
    }
}

#[cfg(all(target_os = "linux", not(test)))]
fn check_mpris_playing() -> bool {
    let run_blocking = || {
        let Ok(conn) = zbus::blocking::Connection::session() else {
            return false;
        };
        let Ok(names_reply) = conn.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        ) else {
            return false;
        };
        let Ok(names): Result<Vec<String>, _> = names_reply.body().deserialize() else {
            return false;
        };
        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.")
                && let Ok(prop_reply) = conn.call_method(
                    Some(name.as_str()),
                    "/org/mpris/MediaPlayer2",
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.mpris.MediaPlayer2.Player", "PlaybackStatus"),
                )
                && let Ok(val) = prop_reply.body().deserialize::<zbus::zvariant::Value>()
                && let Ok(status) = val.downcast::<String>()
                && status == "Playing"
            {
                return true;
            }
        }
        false
    };

    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(run_blocking)
    } else {
        run_blocking()
    }
}

#[cfg(all(not(target_os = "linux"), not(test)))]
fn check_mpris_playing() -> bool {
    false
}

#[cfg(test)]
mod tests;
