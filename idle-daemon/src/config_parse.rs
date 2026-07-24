// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Key/value application for daemon `config.yaml` lines.

use idle_runner::launcher::{is_allowed_saver, sanitize_saver_name};

use crate::config::DaemonConfig;

/// Apply a single `key: value` pair from config.yaml into `config`.
pub(crate) fn apply_config_key(config: &mut DaemonConfig, key: &str, val: &str) {
    match key {
        "idle_timeout_mins" => apply_idle_timeout(config, val),
        "active_saver" => apply_active_saver(config, val),
        "idle_enabled" => {
            if let Ok(b) = val.parse::<bool>() {
                config.idle_enabled = b;
            }
        }
        "gpu_enabled" => apply_gpu_enabled(config, val),
        "show_fps_overlay" => {
            if let Ok(b) = val.parse::<bool>() {
                config.show_fps_overlay = b;
            }
        }
        "render_scale" => apply_render_scale(config, val),
        _ => {}
    }
}

fn apply_idle_timeout(config: &mut DaemonConfig, val: &str) {
    if let Some(n) = val.parse::<u32>().ok().filter(|&n| (1..=240).contains(&n)) {
        config.idle_timeout_mins = n;
    }
}

fn apply_active_saver(config: &mut DaemonConfig, val: &str) {
    if val.is_empty() || val == "none" {
        config.active_saver = None;
    } else if is_allowed_saver(val) {
        config.active_saver = sanitize_saver_name(val).map(|s| s.to_string());
    }
}

fn apply_gpu_enabled(config: &mut DaemonConfig, val: &str) {
    // DEPRECATED (2026): the previous `trance-gpu` crate was renamed to
    // `idle-upscaler` and is now pure CPU code. `gpu_enabled` is a no-op; we
    // accept the value silently for back-compat with existing config.yaml
    // files but ignore it. Logging would be spammy on every daemon start, so
    // no warning is emitted here — the field is documented as deprecated in
    // `config.yaml(5)`.
    let _ = val.parse::<bool>();
    #[allow(deprecated)]
    {
        config.gpu_enabled = false;
    }
}

fn apply_render_scale(config: &mut DaemonConfig, val: &str) {
    if val.is_empty() || val.eq_ignore_ascii_case("null") {
        config.render_scale = None;
    } else if let Some(scale) = val.parse::<f32>().ok().filter(|s| s.is_finite()) {
        config.render_scale = Some(scale.clamp(0.25, 1.0));
    }
}

/// Parse one non-comment config line (`key: value`) into the config.
pub(crate) fn apply_config_line(config: &mut DaemonConfig, line: &str) {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return;
    }
    let Some(idx) = line.find(':') else {
        return;
    };
    let key = line[..idx].trim();
    let val = line[idx + 1..].trim().trim_matches('"').trim_matches('\'');
    apply_config_key(config, key, val);
}
