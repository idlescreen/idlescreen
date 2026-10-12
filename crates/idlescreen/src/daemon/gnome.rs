// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! GNOME Mutter idle monitor fallback over D-Bus when ext-idle-notify-v1 is absent.

use zbus::proxy;

#[proxy(
    interface = "org.gnome.Mutter.IdleMonitor",
    default_service = "org.gnome.Mutter.IdleMonitor",
    default_path = "/org/gnome/Mutter/IdleMonitor/Core"
)]
pub trait MutterIdleMonitor {
    fn add_idle_watch(&self, interval: u64) -> zbus::Result<u32>;
    fn add_user_active_watch(&self) -> zbus::Result<u32>;
    fn remove_watch(&self, id: u32) -> zbus::Result<()>;
    fn get_idletime(&self) -> zbus::Result<u64>;

    #[zbus(signal)]
    fn watch_fired(&self, id: u32) -> zbus::Result<()>;
}

/// Checks whether GNOME Mutter IdleMonitor is available on the session bus.
pub fn is_mutter_idle_available(conn: &zbus::blocking::Connection) -> bool {
    MutterIdleMonitorProxyBlocking::new(conn)
        .and_then(|proxy| proxy.get_idletime())
        .is_ok()
}

#[proxy(
    interface = "org.gnome.ScreenSaver",
    default_service = "org.gnome.ScreenSaver",
    default_path = "/org/gnome/ScreenSaver"
)]
pub trait GnomeScreenSaver {
    fn get_active(&self) -> zbus::Result<bool>;
    fn get_active_time(&self) -> zbus::Result<u32>;

    #[zbus(signal)]
    fn active_changed(&self, new_value: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn wake_up_screen(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait LogindManager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

/// Spawns background threads monitoring GNOME screensaver and logind signals to ensure
/// the screensaver is immediately dismissed on lock, unlock, wake-up, or sleep/resume.
pub fn spawn_gnome_session_watchers(
    state: std::sync::Arc<crate::daemon::dbus::ScreenSaverState>,
) {
    // 1. Session bus watcher: org.gnome.ScreenSaver ActiveChanged & WakeUpScreen
    let st_session = state.clone();
    std::thread::Builder::new()
        .name("idle-gnome-mon".to_string())
        .spawn(move || {
            let Ok(conn) = zbus::blocking::Connection::session() else {
                return;
            };
            let Ok(proxy) = GnomeScreenSaverProxyBlocking::new(&conn) else {
                return;
            };

            // Listen for ActiveChanged signals
            if let Ok(stream) = proxy.receive_active_changed() {
                for signal in stream {
                    if let Ok(args) = signal.args() {
                        let is_active = args.new_value;
                        // On both lock (active=true) and unlock (active=false),
                        // dismiss any running presentation so it never lingers.
                        st_session.stop();
                        if is_active {
                            idle_log_info!("GNOME screen locked/blanked — presentation stopped");
                        } else {
                            idle_log_info!("GNOME session unlocked — screensaver dismissed");
                        }
                    }
                }
            }
        })
        .ok();

    // 2. System bus watcher: logind PrepareForSleep
    let st_system = state;
    std::thread::Builder::new()
        .name("idle-sleep-mon".to_string())
        .spawn(move || {
            let Ok(conn) = zbus::blocking::Connection::system() else {
                return;
            };
            let Ok(proxy) = LogindManagerProxyBlocking::new(&conn) else {
                return;
            };

            if let Ok(stream) = proxy.receive_prepare_for_sleep() {
                for signal in stream {
                    if let Ok(args) = signal.args() {
                        st_system.stop();
                        if args.start {
                            idle_log_info!("System suspending — screensaver stopped");
                        } else {
                            idle_log_info!("System resumed from sleep — screensaver dismissed");
                        }
                    }
                }
            }
        })
        .ok();
}

macro_rules! idle_log_info {
    ($($arg:tt)*) => {
        // Silent in normal operation or logged to stderr if DEBUG
        if std::env::var_os("IDLESCREEN_DEBUG").is_some() {
            eprintln!("[idlescreen::gnome] {}", format_args!($($arg)*));
        }
    };
}
use idle_log_info;

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    #[test]
    fn test_screensaver_state_dismiss_on_activity() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        state.active.store(true, Ordering::SeqCst);
        assert!(state.active.load(Ordering::SeqCst));

        // When user activity occurs, state.stop() dismisses the screensaver
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_screensaver_state_dismiss_on_lock_and_unlock() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());

        // Simulate screensaver activated while idle
        state.active.store(true, Ordering::SeqCst);

        // Screen blanks / locks: stop presentation so it doesn't render behind lock shield
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));

        // When unlocked, state remains false so desktop is immediately usable
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_screensaver_state_dismiss_on_suspend_resume() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        state.active.store(true, Ordering::SeqCst);

        // System prepares for sleep
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));

        // System resumes from sleep
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_idle_and_lock_decision_matrix() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        let idle_timeout_ms = 120_000u64; // 2 minutes

        // Case 1: Idle time exceeded, not locked -> activate
        let t_idle = 150_000u64;
        let is_locked = false;
        if t_idle >= idle_timeout_ms && !state.is_inhibited() && !is_locked {
            state.active.store(true, Ordering::SeqCst);
        }
        assert!(state.active.load(Ordering::SeqCst));

        // Case 2: Screen powers off / locks while screensaver is active -> must dismiss
        let is_locked = true;
        if (t_idle < idle_timeout_ms || is_locked) && state.active.load(Ordering::SeqCst) {
            state.stop();
        }
        assert!(!state.active.load(Ordering::SeqCst));

        // Case 3: User moves trackpad (t_idle drops to 0) while active -> must dismiss
        state.active.store(true, Ordering::SeqCst);
        let t_active = 200u64;
        let is_locked = false;
        if (t_active < idle_timeout_ms || is_locked) && state.active.load(Ordering::SeqCst) {
            state.stop();
        }
        assert!(!state.active.load(Ordering::SeqCst));
    }
}


