// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Sets the active screensaver across D-Bus, control socket, or configuration.

use crate::ascii::effects::EffectKind;
use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::ControlClient;

pub const ALL_KNOWN_SAVERS: &[&str] = &[
    "ascii",
    "superfedora",
    "aurora",
    "beams",
    "bursts",
    "chaos",
    "cosmos",
    "glyphs",
    "gnats",
    "hearth",
    "radar",
    "ripple",
    "storm",
];

pub fn execute(saver_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let Some(saver) = saver_arg else {
        eprintln!("Usage: idlescreen set <saver_name>");
        eprintln!("Available savers: {}", ALL_KNOWN_SAVERS.join(", "));
        return Err("missing screensaver name argument".into());
    };

    let saver_clean = saver.trim().to_lowercase();
    let is_known = ALL_KNOWN_SAVERS.contains(&saver_clean.as_str())
        || EffectKind::parse(&saver_clean).is_some();
    if !is_known {
        eprintln!("Unknown screensaver: '{saver}'");
        eprintln!("Available savers: {}", ALL_KNOWN_SAVERS.join(", "));
        return Err(format!("unknown screensaver '{saver}'").into());
    }

    if saver_clean == "superfedora" && !crate::ascii::sys_info::is_superfedora_machine() {
        eprintln!("error: screensaver 'superfedora' is locked.");
        eprintln!(
            "It only unlocks when IdleScreen is installed and run on a SuperFedora workstation."
        );
        eprintln!("Visit https://superfedora.github.io for installation instructions.");
        return Err("screensaver 'superfedora' is locked on this machine".into());
    }

    let mut notified_daemon = false;

    if let Ok(conn) = zbus::blocking::Connection::session()
        && let Ok(proxy) = crate::daemon::dbus::IdleClientProxyBlocking::new(&conn)
        && proxy.set_saver(&saver_clean).is_ok()
    {
        println!("Active screensaver set to '{saver_clean}' (via D-Bus)");
        notified_daemon = true;
    }

    if !notified_daemon && let Ok(resp) = ControlClient::send_command(&format!("set {saver_clean}"))
    {
        println!("Active screensaver set to '{saver_clean}' (via control socket: {resp})");
        notified_daemon = true;
    }

    let toml_path = resolve_config_dir().join("config.toml");
    let mut config = Config::load_or_migrate();
    config.saver.active = Some(saver_clean.clone());
    let _ = config.save_to_path(&toml_path);

    if !notified_daemon {
        println!("Active screensaver set to '{saver_clean}' (saved to config)");
    }

    Ok(())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn test_all_known_savers_contains_ascii_and_11_extras() {
        assert_eq!(ALL_KNOWN_SAVERS.len(), 13);
        assert!(ALL_KNOWN_SAVERS.contains(&"ascii"));
        assert!(ALL_KNOWN_SAVERS.contains(&"superfedora"));
        assert!(ALL_KNOWN_SAVERS.contains(&"storm"));
        assert!(ALL_KNOWN_SAVERS.contains(&"aurora"));
    }

    #[test]
    fn test_superfedora_unlock_gate_enforcement() {
        // SAFETY: unit test sets and restores scoped environment variable.
        unsafe {
            std::env::set_var("IDLE_SUPERFEDORA", "1");
        }
        assert!(crate::ascii::sys_info::is_superfedora_machine());
        // SAFETY: unit test restores environment variable.
        unsafe {
            std::env::remove_var("IDLE_SUPERFEDORA");
        }
    }
}
