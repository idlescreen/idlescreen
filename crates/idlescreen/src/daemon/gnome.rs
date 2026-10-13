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

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
pub trait LogindSession {
    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unlock(&self) -> zbus::Result<()>;
}

#[derive(Clone, Debug)]
pub struct WakeHandle {
    pub fd: std::sync::Arc<std::sync::atomic::AtomicI32>,
}

impl WakeHandle {
    pub fn new(fd: std::os::fd::RawFd) -> Self {
        Self {
            fd: std::sync::Arc::new(std::sync::atomic::AtomicI32::new(fd)),
        }
    }

    pub fn raw_fd(&self) -> std::os::fd::RawFd {
        self.fd.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn close(&self) {
        let fd = self.fd.swap(-1, std::sync::atomic::Ordering::SeqCst);
        if fd >= 0 {
            unsafe { libc::close(fd) };
        }
    }

    pub fn wake(&self) {
        let fd = self.fd.load(std::sync::atomic::Ordering::SeqCst);
        if fd >= 0 {
            let val: u64 = 1;
            unsafe {
                libc::write(
                    fd,
                    (&raw const val).cast::<libc::c_void>(),
                    std::mem::size_of::<u64>(),
                );
            }
        }
    }
}

/// Spawns background thread monitoring GNOME Mutter IdleMonitor watch signals.
/// Returns true if idle and active watches were successfully registered over D-Bus.
pub fn spawn_mutter_idle_watcher(
    state: std::sync::Arc<crate::daemon::dbus::ScreenSaverState>,
    idle_timeout_ms: u64,
    wake: WakeHandle,
) -> bool {
    let Ok(conn) = zbus::blocking::Connection::session() else {
        return false;
    };
    let Ok(proxy) = MutterIdleMonitorProxyBlocking::new(&conn) else {
        return false;
    };
    let Ok(idle_watch_id) = proxy.add_idle_watch(idle_timeout_ms) else {
        return false;
    };
    let Ok(active_watch_id) = proxy.add_user_active_watch() else {
        return false;
    };

    let st = state;
    std::thread::Builder::new()
        .name("idle-mutter-mon".to_string())
        .spawn(move || {
            if let Ok(stream) = proxy.receive_watch_fired() {
                for signal in stream {
                    if let Ok(args) = signal.args() {
                        if args.id == idle_watch_id {
                            if !st.is_inhibited() {
                                st.active.store(true, std::sync::atomic::Ordering::SeqCst);
                                wake.wake();
                            }
                        } else if args.id == active_watch_id
                            && st.active.load(std::sync::atomic::Ordering::SeqCst)
                        {
                            st.stop();
                            wake.wake();
                        }
                    }
                }
            }
        })
        .is_ok()
}

/// Spawns background threads monitoring GNOME screensaver and logind signals to ensure
/// the screensaver is immediately dismissed on lock, unlock, wake-up, or sleep/resume.
pub fn spawn_gnome_session_watchers(
    state: std::sync::Arc<crate::daemon::dbus::ScreenSaverState>,
    wake: WakeHandle,
) {
    // 1. Session bus watcher: org.gnome.ScreenSaver ActiveChanged & WakeUpScreen
    let st_session = state.clone();
    let wake_session = wake.clone();
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
                        wake_session.wake();
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

    // 2. System bus watcher: logind Session Lock / Unlock (for loginctl lock-session on Hyprland/Sway/COSMIC)
    let st_lock = state.clone();
    let wake_lock = wake.clone();
    std::thread::Builder::new()
        .name("idle-lock-mon".to_string())
        .spawn(move || {
            let Ok(conn) = zbus::blocking::Connection::system() else {
                return;
            };
            let Ok(proxy) = LogindSessionProxyBlocking::new(&conn) else {
                return;
            };
            if let Ok(stream) = proxy.receive_lock() {
                for _ in stream {
                    st_lock.stop();
                    wake_lock.wake();
                    idle_log_info!("logind Session.Lock — presentation stopped immediately");
                }
            }
        })
        .ok();

    let st_unlock = state.clone();
    let wake_unlock = wake.clone();
    std::thread::Builder::new()
        .name("idle-unlock-mon".to_string())
        .spawn(move || {
            let Ok(conn) = zbus::blocking::Connection::system() else {
                return;
            };
            let Ok(proxy) = LogindSessionProxyBlocking::new(&conn) else {
                return;
            };
            if let Ok(stream) = proxy.receive_unlock() {
                for _ in stream {
                    st_unlock.stop();
                    wake_unlock.wake();
                    idle_log_info!("logind Session.Unlock — screensaver dismissed");
                }
            }
        })
        .ok();

    // 3. System bus watcher: logind PrepareForSleep
    let st_system = state;
    let wake_system = wake;
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
                        wake_system.wake();
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
