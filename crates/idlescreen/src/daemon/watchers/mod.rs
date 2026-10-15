// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Event-driven idle and session watchers across Wayland (ext-idle-notify-v1),
//! GNOME Mutter, systemd-logind, elogind, and ConsoleKit.

#![allow(unsafe_code)]

pub mod media_inhibit;
pub mod mutter;
pub mod session;
pub mod wayland_idle;

pub use mutter::{
    GnomeScreenSaverProxyBlocking, MutterIdleMonitorProxyBlocking, is_mutter_idle_available,
    spawn_mutter_idle_watcher,
};
pub use session::{
    LogindManagerProxyBlocking, LogindSessionProxyBlocking, spawn_gnome_session_watchers,
};
pub use wayland_idle::spawn_wayland_idle_watcher;

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct WakeHandle {
    fd: Option<Arc<OwnedFd>>,
}

impl WakeHandle {
    pub fn new_eventfd() -> Self {
        // SAFETY: `eventfd` with `EFD_CLOEXEC | EFD_NONBLOCK` returns a new fd or -1.
        let raw = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if raw >= 0 {
            // SAFETY: `raw` is a freshly created valid eventfd descriptor.
            Self {
                fd: Some(Arc::new(unsafe { OwnedFd::from_raw_fd(raw) })),
            }
        } else {
            Self { fd: None }
        }
    }

    pub fn new(raw_fd: RawFd) -> Self {
        if raw_fd >= 0 {
            // SAFETY: Caller transfers ownership of valid open `raw_fd`.
            Self {
                fd: Some(Arc::new(unsafe { OwnedFd::from_raw_fd(raw_fd) })),
            }
        } else {
            Self { fd: None }
        }
    }

    pub fn raw_fd(&self) -> RawFd {
        self.fd.as_ref().map_or(-1, AsRawFd::as_raw_fd)
    }

    pub fn is_valid(&self) -> bool {
        self.fd.is_some()
    }

    pub fn drain(&self) {
        if let Some(ref f) = self.fd {
            let mut buf = 0u64;
            // SAFETY: `f` is a valid non-blocking eventfd and `buf` is an 8-byte `u64`.
            unsafe {
                let _ = libc::read(
                    f.as_raw_fd(),
                    (&raw mut buf).cast::<libc::c_void>(),
                    std::mem::size_of::<u64>(),
                );
            }
        }
    }

    pub fn wake(&self) {
        if let Some(ref f) = self.fd {
            let val: u64 = 1;
            // SAFETY: `f` is a valid non-blocking eventfd and `val` is an 8-byte `u64`.
            unsafe {
                let _ = libc::write(
                    f.as_raw_fd(),
                    (&raw const val).cast::<libc::c_void>(),
                    std::mem::size_of::<u64>(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::media_inhibit::{
        extract_mpris_player_name, filter_pactl_sink_inputs, has_active_alsa_playback_in,
        has_active_mic_capture, is_alsa_status_running, parse_alsa_owner_pid,
        parse_hyprland_inhibiting_clients, parse_logind_inhibitors_json, parse_pactl_muted_sinks,
        parse_pactl_sink_inputs,
    };
    use std::sync::atomic::Ordering;

    #[test]
    fn test_screensaver_state_dismiss_on_activity() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        state.active.store(true, Ordering::SeqCst);
        assert!(state.active.load(Ordering::SeqCst));
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_screensaver_state_dismiss_on_lock_and_unlock() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        state.active.store(true, Ordering::SeqCst);
        state.stop();
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_idle_and_lock_decision_matrix() {
        let state = std::sync::Arc::new(crate::daemon::dbus::ScreenSaverState::new());
        let idle_timeout_ms = 120_000u64;
        let t_idle = 150_000u64;
        let is_locked = false;
        if t_idle >= idle_timeout_ms && !state.is_inhibited() && !is_locked {
            state.active.store(true, Ordering::SeqCst);
        }
        assert!(state.active.load(Ordering::SeqCst));

        let is_locked = true;
        if (t_idle < idle_timeout_ms || is_locked) && state.active.load(Ordering::SeqCst) {
            state.stop();
        }
        assert!(!state.active.load(Ordering::SeqCst));
    }

    #[test]
    fn test_alsa_and_pactl_and_mpris_media_detection() {
        assert!(is_alsa_status_running(
            "state: RUNNING\nowner_pid   : 2919\n"
        ));
        assert_eq!(
            parse_alsa_owner_pid("state: RUNNING\nowner_pid   : 2919\n"),
            Some(2919)
        );
        assert_eq!(parse_alsa_owner_pid("closed\n"), None);
        assert!(!is_alsa_status_running("closed\n"));
        assert!(!is_alsa_status_running("state: PREPARED\n"));

        let tmp = tempfile::tempdir().unwrap();
        let cap_sub = tmp.path().join("card0/pcm0c/sub0");
        std::fs::create_dir_all(&cap_sub).unwrap();
        std::fs::write(cap_sub.join("status"), "state: RUNNING\n").unwrap();
        assert_eq!(has_active_alsa_playback_in(tmp.path()), None);

        let play_sub = tmp.path().join("card0/pcm0p/sub0");
        std::fs::create_dir_all(&play_sub).unwrap();
        std::fs::write(play_sub.join("status"), "state: RUNNING\n").unwrap();
        assert_eq!(
            has_active_alsa_playback_in(tmp.path()).as_deref(),
            Some("card0/pcm0p/sub0")
        );

        let pactl_out = "Sink Input #1\n\tCorked: no\n\tMute: no\n\tProperties:\n\t\tapplication.name = \"Firefox\"\n\t\tmedia.name = \"South Park\"\nSink Input #2\n\tCorked: yes\n\tMute: no\n\tProperties:\n\t\tapplication.name = \"mpv\"\nSink Input #3\n\tCorked: no\n\tMute: yes\n\tProperties:\n\t\tapplication.name = \"MutedApp\"\nSink Input #4\n\tCorked: no\n\tMute: no\n\tProperties:\n\t\tnode.name = \"bell\"\n\t\tmedia.role = \"event\"\n";
        let streams = parse_pactl_sink_inputs(pactl_out);
        assert_eq!(
            streams,
            vec![("Firefox".to_string(), "Playing: South Park".to_string())]
        );
        assert!(parse_pactl_sink_inputs("").is_empty());
        assert_eq!(
            extract_mpris_player_name("org.mpris.MediaPlayer2.firefox.instance_1_107470"),
            "firefox"
        );

        let state = crate::daemon::dbus::ScreenSaverState::new();
        state
            .inhibitors
            .lock()
            .unwrap()
            .push(crate::daemon::dbus::InhibitEntry {
                cookie: 1,
                app_name: "Firefox".into(),
                reason: "Playing video".into(),
                sender: None,
            });
        let (resp, _) = state.dispatch_control_command("start");
        assert_eq!(resp, "OK: inhibited (skipped)");
        assert!(!state.active.load(Ordering::SeqCst));

        let (resp_force, _) = state.dispatch_control_command("start --force");
        assert_eq!(resp_force, "OK");
        assert!(state.active.load(Ordering::SeqCst));

        let logind_json = "{\n\"type\" : \"a(ssssuu)\",\n\"data\" : [[[\n\"idle:sleep\",\n\"firefox\",\n\"video-playing\",\n\"block\",\n1000,\n123\n],[\n\"sleep\",\n\"hypridle\",\n\"delay sleep\",\n\"delay\",\n1000,\n456\n]]]\n}";
        assert_eq!(
            parse_logind_inhibitors_json(logind_json),
            vec![("firefox".to_string(), "video-playing".to_string())]
        );

        let hypr_json = "[{\"address\": \"0x1\",\n\"mapped\": true,\n\"class\": \"mpv\",\n\"title\": \"Movie.mkv\",\n\"inhibitingIdle\": true\n},{\"address\": \"0x2\",\n\"mapped\": true,\n\"class\": \"kitty\",\n\"title\": \"shell\",\n\"inhibitingIdle\": false\n}]";
        assert_eq!(
            parse_hyprland_inhibiting_clients(hypr_json),
            vec![(
                "mpv".to_string(),
                "Window idle inhibit (Movie.mkv)".to_string()
            )]
        );
    }

    #[test]
    fn test_pactl_browser_webaudio_and_muted_sink_filtering() {
        let sinks = "Sink #6657\n\tMute: no\nSink #6658\n\tMute: yes\n";
        assert_eq!(parse_pactl_muted_sinks(sinks), vec![6658]);
        assert!(has_active_mic_capture(
            "Source Output #1\n\tCorked: no\n\tMute: no\n"
        ));
        assert!(!has_active_mic_capture(
            "Source Output #1\n\tCorked: no\n\tMute: no\n\t\tnode.name = \"Peak detect\"\n"
        ));
        assert!(!has_active_mic_capture(
            "Source Output #1\n\tCorked: no\n\tMute: no\n\t\tpulse.corked = \"true\"\n"
        ));
        assert!(!has_active_mic_capture(
            "Source Output #1\n\tCorked: no\n\tMute: no\n\t\tnode.target = \"alsa_output.sink.monitor\"\n"
        ));
        assert!(!has_active_mic_capture(
            "Source Output #1\n\tCorked: no\n\tMute: no\n\t\tapplication.name = \"cava\"\n"
        ));

        let si = "Sink Input #127133\n\tSink: 6658\n\tCorked: no\n\tMute: no\n\tProperties:\n\t\tapplication.name = \"Firefox\"\n\t\tmedia.name = \"SuperFedora\"\nSink Input #127134\n\tSink: 6657\n\tCorked: no\n\tMute: no\n\tProperties:\n\t\tapplication.name = \"mpv\"\n\t\tmedia.name = \"Song.flac\"\n";
        assert!(
            filter_pactl_sink_inputs(&si[..si.find("Sink Input #127134").unwrap()], &[], false)
                .is_empty()
        );
        assert_eq!(
            filter_pactl_sink_inputs(&si[..si.find("Sink Input #127134").unwrap()], &[], true)
                .len(),
            1
        );
        assert!(
            filter_pactl_sink_inputs(&si[..si.find("Sink Input #127134").unwrap()], &[6658], true)
                .is_empty()
        );
        assert_eq!(
            filter_pactl_sink_inputs(si, &[6658], false),
            vec![("mpv".to_string(), "Playing: Song.flac".to_string())]
        );
    }
}
