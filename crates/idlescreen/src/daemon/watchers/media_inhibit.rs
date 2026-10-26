// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! On-demand active media, audio stream, and system idle inhibitor detection
//! across ALSA (`/proc/asound`), PipeWire/PulseAudio (`pactl`), MPRIS D-Bus,
//! Wayland/Hyprland surface idle inhibitors, and systemd-logind.

use std::fs;
use std::path::Path;
use std::process::Command;

use super::capped_cmd::{self, DEFAULT_TIMEOUT};

pub const MEDIA_COOKIE_BASE: u32 = 9000;

/// Checks if an ALSA `/proc/asound/card*/pcm*p/sub*/status` payload reports active DMA playback.
pub fn is_alsa_status_running(content: &str) -> bool {
    content.lines().any(|line| line.trim() == "state: RUNNING")
}

/// Extracts `owner_pid` from an ALSA `/proc/asound/card*/pcm*p/sub*/status` payload.
pub fn parse_alsa_owner_pid(content: &str) -> Option<u32> {
    content.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k.trim() == "owner_pid").then(|| v.trim().parse::<u32>().ok())?
    })
}

fn is_sound_server_pid(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|c| {
        matches!(
            c.trim(),
            "pipewire" | "pipewire-pulse" | "pulseaudio" | "wireplumber"
        )
    })
}

/// Scans `/proc/asound/card*/pcm*p/sub*/status` (playback streams only, ignoring `pcm*c` capture).
pub fn has_active_alsa_playback_in(asound_root: &Path) -> Option<String> {
    has_active_alsa_playback_filtered(asound_root, false)
}

#[rustfmt::skip]
pub fn has_active_alsa_playback_filtered(asound_root: &Path, ignore_sound_servers: bool) -> Option<String> {
    for card in fs::read_dir(asound_root).ok()?.flatten() {
        let card_str = card.file_name().to_string_lossy().into_owned();
        if !card_str.starts_with("card") { continue; }
        let Ok(pcms) = fs::read_dir(card.path()) else { continue };
        for pcm in pcms.flatten() {
            let pcm_str = pcm.file_name().to_string_lossy().into_owned();
            if !pcm_str.starts_with("pcm") || !pcm_str.ends_with('p') { continue; }
            let Ok(subs) = fs::read_dir(pcm.path()) else { continue };
            for sub in subs.flatten() {
                let sub_str = sub.file_name().to_string_lossy().into_owned();
                if !sub_str.starts_with("sub") { continue; }
                if let Ok(content) = fs::read_to_string(sub.path().join("status")) && is_alsa_status_running(&content) {
                    if ignore_sound_servers && parse_alsa_owner_pid(&content).is_some_and(is_sound_server_pid) {
                        continue;
                    }
                    return Some(format!("{card_str}/{pcm_str}/{sub_str}"));
                }
            }
        }
    }
    None
}

#[rustfmt::skip]
pub fn is_browser_stream(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["firefox", "librewolf", "zen", "floorp", "waterfox", "chromium", "chrome", "brave", "vivaldi", "msedge", "opera", "thorium", "epiphany", "webkit"]
        .iter()
        .any(|b| lower.contains(b))
}

#[rustfmt::skip]
pub fn parse_pactl_muted_sinks(sinks_out: &str) -> Vec<u32> {
    sinks_out.split("Sink #").skip(1).filter_map(|block| {
        let id = block.lines().next()?.trim().parse::<u32>().ok()?;
        block.lines().any(|l| l.trim().strip_prefix("Mute:").is_some_and(|v| v.trim().eq_ignore_ascii_case("yes"))).then_some(id)
    }).collect()
}

#[rustfmt::skip]
pub fn has_active_mic_capture(source_outputs_out: &str) -> bool {
    source_outputs_out.split("Source Output #").skip(1).any(|block| {
        let (mut corked, mut muted, mut ignore) = (false, false, false);
        for line in block.lines().map(str::trim) {
            if let Some(v) = line.strip_prefix("Corked:") { corked = v.trim().eq_ignore_ascii_case("yes"); }
            else if let Some(v) = line.strip_prefix("Mute:") { muted = v.trim().eq_ignore_ascii_case("yes"); }
            else if let Some((k, v)) = line.split_once('=') {
                let (key, val) = (k.trim(), v.trim().trim_matches('"').trim());
                if key == "pulse.corked" && val.eq_ignore_ascii_case("true") { corked = true; }
                else if (key == "media.role" && val.eq_ignore_ascii_case("event")) || (key == "node.name" && val.eq_ignore_ascii_case("peak detect")) || (key == "node.target" && val.ends_with(".monitor")) || (matches!(key, "application.name" | "node.name") && matches!(val.to_ascii_lowercase().as_str(), "cava" | "pavucontrol")) {
                    ignore = true;
                }
            }
        }
        !corked && !muted && !ignore
    })
}

/// Parses `pactl list sink-inputs` output and returns `(app_name, reason)` for uncorked, unmuted streams.
pub fn parse_pactl_sink_inputs(output: &str) -> Vec<(String, String)> {
    parse_pactl_sink_inputs_ext(output, &[], false, false)
}

#[rustfmt::skip]
pub fn filter_pactl_sink_inputs(output: &str, muted_sinks: &[u32], has_mic_capture: bool) -> Vec<(String, String)> {
    parse_pactl_sink_inputs_ext(output, muted_sinks, has_mic_capture, true)
}

#[rustfmt::skip]
fn parse_pactl_sink_inputs_ext(output: &str, muted_sinks: &[u32], has_mic_capture: bool, filter_browser_webaudio: bool) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for block in output.split("Sink Input #").skip(1) {
        let (mut corked, mut muted, mut is_event, mut sink_id) = (false, false, false, None);
        let (mut app_name, mut node_name, mut media_name, mut proc_bin) = (None, None, None, None);
        for line in block.lines().map(str::trim) {
            if let Some(val) = line.strip_prefix("Corked:") { corked = val.trim().eq_ignore_ascii_case("yes"); }
            else if let Some(val) = line.strip_prefix("Mute:") { muted = val.trim().eq_ignore_ascii_case("yes"); }
            else if let Some(val) = line.strip_prefix("Sink:") { sink_id = val.trim().parse::<u32>().ok(); }
            else if let Some((k, v)) = line.split_once('=') {
                let (key, val) = (k.trim(), v.trim().trim_matches('"').trim());
                match key {
                    "application.name" if !val.is_empty() => app_name = Some(val.to_string()),
                    "node.name" if !val.is_empty() => node_name = Some(val.to_string()),
                    "media.name" if !val.is_empty() => media_name = Some(val.to_string()),
                    "application.process.binary" if !val.is_empty() => proc_bin = Some(val.to_string()),
                    "media.role" if val.eq_ignore_ascii_case("event") => is_event = true,
                    "pulse.corked" if val.eq_ignore_ascii_case("true") => corked = true,
                    _ => {}
                }
            }
        }
        if corked || muted || is_event || sink_id.is_some_and(|id| muted_sinks.contains(&id)) { continue; }
        let app = app_name.or(node_name).unwrap_or_else(|| "PipeWire Audio".into());
        let is_browser = is_browser_stream(&app) || proc_bin.as_deref().is_some_and(is_browser_stream);
        let is_webrtc = media_name.as_deref().is_some_and(|m| m.to_ascii_lowercase().contains("webrtc"));
        if filter_browser_webaudio && is_browser && !has_mic_capture && !is_webrtc { continue; }
        let reason = media_name.map_or_else(|| "Active audio stream".into(), |m| format!("Playing: {m}"));
        results.push((app, reason));
    }
    results
}

#[rustfmt::skip]
pub fn extract_mpris_player_name(bus_name: &str) -> String {
    let suffix = bus_name.strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(bus_name);
    suffix.split('.').next().filter(|s| !s.is_empty()).unwrap_or(suffix).to_string()
}

#[rustfmt::skip]
pub fn probe_mpris_playing() -> Vec<(String, String)> {
    let Some(out) = capped_cmd::run_capped(Command::new("busctl").env("LC_ALL", "C").args(["--user", "--timeout=500ms", "list", "--acquired", "--no-legend"]), DEFAULT_TIMEOUT) else { return Vec::new() };
    if !out.status.success() { return Vec::new(); }
    let mut active = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(name) = line.split_whitespace().next() else { continue };
        if !name.starts_with("org.mpris.MediaPlayer2.") || name.ends_with(".playerctld") { continue; }
        let args = ["--user", "--timeout=500ms", "get-property", name, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player", "PlaybackStatus"];
        if let Some(p) = capped_cmd::run_capped(Command::new("busctl").env("LC_ALL", "C").args(args), DEFAULT_TIMEOUT) && p.status.success() && String::from_utf8_lossy(&p.stdout).to_ascii_lowercase().contains("\"playing\"") {
            active.push((format!("{} (MPRIS)", extract_mpris_player_name(name)), "Media playback active".into()));
        }
    }
    active
}

#[rustfmt::skip]
pub fn probe_pactl_playing_opt() -> Option<Vec<(String, String)>> {
    let out = capped_cmd::run_capped(Command::new("pactl").env("LC_ALL", "C").args(["list", "sink-inputs"]), DEFAULT_TIMEOUT)?;
    if !out.status.success() { return None; }
    let sinks_out = capped_cmd::run_capped(Command::new("pactl").env("LC_ALL", "C").args(["list", "sinks"]), DEFAULT_TIMEOUT);
    let muted_sinks = sinks_out.as_ref().filter(|o| o.status.success()).map_or_else(Vec::new, |o| parse_pactl_muted_sinks(&String::from_utf8_lossy(&o.stdout)));
    let src_out = capped_cmd::run_capped(Command::new("pactl").env("LC_ALL", "C").args(["list", "source-outputs"]), DEFAULT_TIMEOUT);
    let has_mic = src_out.as_ref().filter(|o| o.status.success()).is_some_and(|o| has_active_mic_capture(&String::from_utf8_lossy(&o.stdout)));
    Some(filter_pactl_sink_inputs(&String::from_utf8_lossy(&out.stdout), &muted_sinks, has_mic))
}

#[rustfmt::skip]
pub fn parse_hyprland_inhibiting_clients(json_out: &str) -> Vec<(String, String)> {
    let mut active = Vec::new();
    for block in json_out.split("\"address\":") {
        if !block.contains("\"inhibitingIdle\": true") || block.contains("\"mapped\": false") { continue; }
        let (mut class, mut title) = ("wayland-client", "Wayland idle inhibitor");
        for line in block.lines() {
            let t = line.trim().trim_end_matches(',');
            if let Some(v) = t.strip_prefix("\"class\":") {
                let s = v.trim().trim_matches('"');
                if !s.is_empty() { class = s; }
            } else if let Some(v) = t.strip_prefix("\"title\":") {
                let s = v.trim().trim_matches('"');
                if !s.is_empty() { title = s; }
            }
        }
        active.push((class.to_string(), format!("Window idle inhibit ({title})")));
    }
    active
}

pub fn probe_hyprland_idle_inhibitors() -> Vec<(String, String)> {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return Vec::new();
    }
    let Some(out) = capped_cmd::run_capped(
        Command::new("hyprctl").args(["clients", "-j"]),
        DEFAULT_TIMEOUT,
    ) else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    parse_hyprland_inhibiting_clients(&String::from_utf8_lossy(&out.stdout))
}

#[rustfmt::skip]
pub fn parse_logind_inhibitors_json(json_out: &str) -> Vec<(String, String)> {
    let strings: Vec<String> = json_out.lines().filter_map(|line| {
        let t = line.trim().trim_end_matches(',');
        (t.starts_with('"') && t.ends_with('"') && t.len() >= 2 && !t.contains("\" :") && !t.contains("\":")).then(|| t[1..t.len() - 1].to_string())
    }).collect();
    strings.chunks_exact(4).filter(|c| c[3] == "block" && c[0].split(':').any(|w| w.trim() == "idle")).map(|c| (c[1].clone(), c[2].clone())).collect()
}

#[rustfmt::skip]
pub fn probe_logind_idle_inhibitors() -> Vec<(String, String)> {
    let args = ["--system", "--timeout=500ms", "-j", "call", "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager", "ListInhibitors"];
    let Some(out) = capped_cmd::run_capped(Command::new("busctl").env("LC_ALL", "C").args(args), DEFAULT_TIMEOUT) else { return Vec::new() };
    if !out.status.success() { return Vec::new(); }
    parse_logind_inhibitors_json(&String::from_utf8_lossy(&out.stdout))
}

#[rustfmt::skip]
pub fn probe_active_inhibitors() -> Vec<(u32, String, String)> {
    let mut list = Vec::new();
    let mut cookie = MEDIA_COOKIE_BASE;
    for (app, reason) in probe_mpris_playing() { list.push((cookie, app, reason)); cookie += 1; }
    let pactl_res = probe_pactl_playing_opt();
    let pactl_ok = pactl_res.is_some();
    let pactl_streams = pactl_res.unwrap_or_default();
    let has_pactl = !pactl_streams.is_empty();
    for (app, reason) in pactl_streams { list.push((cookie, app, reason)); cookie += 1; }
    if !has_pactl && let Some(pcm) = has_active_alsa_playback_filtered(Path::new("/proc/asound"), pactl_ok) {
        list.push((cookie, "alsa-audio".to_string(), format!("Hardware audio playback active ({pcm})")));
        cookie += 1;
    }
    for (app, reason) in probe_hyprland_idle_inhibitors() { list.push((cookie, app, reason)); cookie += 1; }
    for (who, why) in probe_logind_idle_inhibitors() { list.push((cookie, who, why)); cookie += 1; }
    list
}

/// Checks whether any system media, audio stream, or idle inhibitor is active.
pub fn is_media_or_system_inhibited() -> bool {
    !probe_active_inhibitors().is_empty()
}
