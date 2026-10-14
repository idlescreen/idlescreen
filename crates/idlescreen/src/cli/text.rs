// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Sets or queries active ASCII screensaver text targets across D-Bus, control socket, and config.

use crate::ascii::state::TriRotationState;
use crate::daemon::config::{Config, parse_targets_from_mode, resolve_config_dir};
use crate::daemon::control_sock::ControlClient;

pub fn execute(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::load_or_migrate();
    let state = TriRotationState::default();

    if args.is_empty() || (args.len() == 1 && args[0].eq_ignore_ascii_case("status")) {
        print_status(&config, &state);
        return Ok(());
    }

    let mut raw_targets: Vec<String> = Vec::new();
    for arg in args {
        for part in arg.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                raw_targets.push(trimmed.to_string());
            }
        }
    }

    if raw_targets.is_empty() {
        print_status(&config, &state);
        return Ok(());
    }

    let mut state_with_targets = TriRotationState::default();
    state_with_targets.set_targets(raw_targets.clone());
    let resolved_words: Vec<String> = state_with_targets
        .targets()
        .iter()
        .map(|t| state_with_targets.resolve_target_text(t))
        .collect();

    let mut notified_via = None;

    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
        && proxy.set_text_targets(&raw_targets).is_ok()
    {
        notified_via = Some("D-Bus");
    }

    if notified_via.is_none() {
        let cmd = format!("text_targets {}", raw_targets.join(","));
        if ControlClient::send_command(&cmd).is_ok() {
            notified_via = Some("control socket");
        }
    }

    let toml_path = resolve_config_dir().join("config.toml");
    config.saver.ascii.targets.clone_from(&raw_targets);
    config.saver.ascii.mode = raw_targets.join(",");
    if config.saver.active.is_none() {
        config.saver.active = Some("ascii".to_string());
    }

    let write_err = config.save_to_path(&toml_path).err().map(|e| e.to_string());

    let note = match (notified_via, write_err.as_ref()) {
        (Some(via), None) => format!(" (daemon updated via {via})"),
        (Some(via), Some(err)) => {
            format!(" (daemon updated via {via}; warning: failed to write config: {err})")
        }
        (None, None) => " (saved to config)".to_string(),
        (None, Some(err)) => format!(" (warning: failed to write config: {err})"),
    };
    println!(
        "ASCII text targets set to: {}{}",
        raw_targets.join(", "),
        note
    );
    println!("Resolved brand words: {}", resolved_words.join(", "));
    if resolved_words.len() > 1 {
        let seq = resolved_words.join(" -> ");
        println!("Scene handoff sequence: {seq} -> {}", resolved_words[0]);
    } else if let Some(single) = resolved_words.first() {
        println!("Single target active: Every scene runs with '{single}'");
    }

    Ok(())
}

fn print_status(config: &Config, state: &TriRotationState) {
    let mut current_targets = config.saver.ascii.targets.clone();
    if current_targets.is_empty() {
        current_targets = parse_targets_from_mode(&config.saver.ascii.mode);
    }

    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
        && let Ok(live) = proxy.get_text_targets()
        && !live.is_empty()
    {
        current_targets = live;
    }

    let mut temp_state = TriRotationState::default();
    temp_state.set_targets(current_targets.clone());
    let resolved: Vec<String> = temp_state
        .targets()
        .iter()
        .map(|t| temp_state.resolve_target_text(t))
        .collect();

    println!("IdleScreen ASCII Text Configuration:");
    println!("  Active targets: [{}]", current_targets.join(", "));
    println!("  Resolved brand words: [{}]", resolved.join(", "));
    if resolved.len() > 1 {
        println!("  Sequence: {} -> {}", resolved.join(" -> "), resolved[0]);
    } else if let Some(single) = resolved.first() {
        println!("  Sequence: Single text ({single}) on all scenes");
    }
    println!("  Host OS: {}", state.os_name);
    println!("  Desktop Env: {}", state.de_name);
    println!("  Kernel: {}", state.kernel_version);
}
