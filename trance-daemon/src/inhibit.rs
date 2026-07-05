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
    logind_cache: Mutex<(bool, std::time::Instant)>,
}

impl InhibitorState {
    pub fn new() -> Self {
        Self {
            inhibitors: Mutex::new(Vec::new()),
            last_cookie: AtomicU32::new(0),
            logind_cache: Mutex::new((
                false,
                std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(5))
                    .unwrap_or_else(std::time::Instant::now),
            )),
        }
    }

    pub fn is_inhibited(&self) -> bool {
        if !self.inhibitors.lock().unwrap().is_empty() {
            return true;
        }

        let mut cache = self.logind_cache.lock().unwrap();
        if cache.1.elapsed() >= std::time::Duration::from_secs(2) {
            cache.0 = check_logind_inhibited();
            cache.1 = std::time::Instant::now();
        }
        cache.0
    }

    pub fn add(
        &self,
        application_name: String,
        reason: String,
        client: UniqueName<'static>,
    ) -> Result<u32, &'static str> {
        let mut inhibitors = self.inhibitors.lock().unwrap();
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

    /// Remove an inhibitor only when `cookie` belongs to `client`.
    pub fn remove_for_client(&self, cookie: u32, client: &UniqueName<'_>) -> bool {
        let mut inhibitors = self.inhibitors.lock().unwrap();
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
        let mut inhibitors = self.inhibitors.lock().unwrap();
        inhibitors.retain(|entry| entry.client != *client);
    }
}

#[cfg(target_os = "linux")]
type LogindInhibitorInfo = (String, String, String, String, u32, u32);

#[cfg(target_os = "linux")]
fn check_logind_inhibited() -> bool {
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
}

#[cfg(not(target_os = "linux"))]
fn check_logind_inhibited() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::names::UniqueName;

    fn client(name: &str) -> UniqueName<'static> {
        UniqueName::try_from(name.to_string()).unwrap()
    }

    #[test]
    fn inhibitor_state_starts_uninhibited() {
        let s = InhibitorState::new();
        assert!(!s.is_inhibited());
    }

    #[test]
    fn add_inhibitor_marks_inhibited() {
        let s = InhibitorState::new();
        let c = client(":test.app.Inhibitor");
        let cookie = s
            .add("app".to_string(), "reason".to_string(), c.clone())
            .unwrap();
        assert!(cookie > 0);
        assert!(s.is_inhibited());
    }

    #[test]
    fn remove_for_client_clears_inhibition() {
        let s = InhibitorState::new();
        let c = client(":test.app.Inhibitor");
        let cookie = s
            .add("app".to_string(), "reason".to_string(), c.clone())
            .unwrap();
        assert!(s.remove_for_client(cookie, &c));
        assert!(!s.is_inhibited());
    }

    #[test]
    fn remove_for_client_wrong_cookie_returns_false() {
        let s = InhibitorState::new();
        let c = client(":test.app.Inhibitor");
        let _ = s.add("app".to_string(), "reason".to_string(), c.clone());
        assert!(!s.remove_for_client(9999, &c));
        assert!(s.is_inhibited());
    }

    #[test]
    fn remove_for_client_wrong_client_returns_false() {
        let s = InhibitorState::new();
        let c1 = client(":test.one.Client");
        let c2 = client(":test.two.Client");
        let cookie = s
            .add("app".to_string(), "reason".to_string(), c1.clone())
            .unwrap();
        assert!(!s.remove_for_client(cookie, &c2));
        assert!(s.is_inhibited());
    }

    #[test]
    fn remove_client_clears_all_for_that_client() {
        let s = InhibitorState::new();
        let c1 = client(":test.one.Client");
        let c2 = client(":test.two.Client");
        let _ = s
            .add("app".to_string(), "reason1".to_string(), c1.clone())
            .unwrap();
        let _ = s
            .add("app".to_string(), "reason2".to_string(), c1.clone())
            .unwrap();
        let _ = s
            .add("app".to_string(), "reason3".to_string(), c2.clone())
            .unwrap();
        s.remove_client(&c1);
        assert!(s.is_inhibited()); // c2 still holds an inhibitor
        s.remove_client(&c2);
        assert!(!s.is_inhibited());
    }

    #[test]
    fn cookies_are_unique_and_increasing() {
        let s = InhibitorState::new();
        let c = client(":test.app.Cookie");
        let k1 = s.add("a".to_string(), "r".to_string(), c.clone()).unwrap();
        let k2 = s.add("a".to_string(), "r".to_string(), c.clone()).unwrap();
        let k3 = s.add("a".to_string(), "r".to_string(), c.clone()).unwrap();
        assert!(k1 < k2);
        assert!(k2 < k3);
    }

    #[test]
    fn add_rejects_when_at_capacity_for_one_client() {
        let s = InhibitorState::new();
        let c = client(":test.app.Capacity");
        for i in 0..32 {
            assert!(
                s.add("a".to_string(), format!("r{i}"), c.clone()).is_ok(),
                "expected add {i} to succeed"
            );
        }
        // 33rd should be rejected (per-cap of 32)
        assert!(s.add("a".to_string(), "r".to_string(), c.clone()).is_err());
    }
}
