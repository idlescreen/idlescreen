// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Sets the active screensaver across D-Bus, control socket, or configuration.

use crate::ascii::effects::EffectKind;
use crate::daemon::config::{Config, resolve_config_dir};
use crate::daemon::control_sock::ControlClient;

#[rustfmt::skip]
pub const ALL_KNOWN_SAVERS: &[&str] = &[
    "ascii", "superfedora", "tour",
    "aurora", "beams", "binarypath", "blackhole", "bouncyballs",
    "breakout", "bubbles", "burn", "chaos", "colorshift",
    "cosmos", "crumble", "decrypt", "errorcorrect", "expand",
    "fireworks", "highlight", "laseretch", "life", "matrix",
    "metaballs", "middleout", "mmx", "moire", "mystify", "orbittingvolley",
    "overflow", "pipes", "plasma", "pour", "print",
    "radar", "rain", "randomsequence", "rings", "ripple",
    "scattered", "slice", "slide", "smoke", "spotlights",
    "spray", "swarm", "sweep", "synthgrid", "tetris",
    "thunderstorm", "tunnel", "unstable", "vhstape", "waves",
    "wipe", "zero",
];

pub fn execute(saver_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let Some(saver) = saver_arg else {
        eprintln!("Usage: idlescreen set <saver_name>");
        eprintln!("Available savers: {}", ALL_KNOWN_SAVERS.join(", "));
        return Err("missing screensaver name argument".into());
    };

    let saver_clean = saver.trim().to_lowercase();
    let canonical = if saver_clean == "ascii" || saver_clean == "superfedora" {
        saver_clean.clone()
    } else if saver_clean == "tour" || saver_clean == "new" {
        "tour".to_string()
    } else if let Some(kind) = EffectKind::parse(&saver_clean) {
        kind.name().to_string()
    } else {
        eprintln!("Unknown screensaver: '{saver}'");
        eprintln!("Available savers: {}", ALL_KNOWN_SAVERS.join(", "));
        return Err(format!("unknown screensaver '{saver}'").into());
    };

    if canonical == "superfedora" && !crate::ascii::sys_info::is_superfedora_machine() {
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
        && proxy.set_saver(&canonical).is_ok()
    {
        println!("Active screensaver set to '{canonical}' (via D-Bus)");
        notified_daemon = true;
    }

    if !notified_daemon && let Ok(resp) = ControlClient::send_command(&format!("set {canonical}")) {
        println!("Active screensaver set to '{canonical}' (via control socket: {resp})");
        notified_daemon = true;
    }

    let toml_path = resolve_config_dir().join("config.toml");
    // Refuse to persist over a config we could not parse: writing defaults
    // would silently discard the user's settings.
    let mut config = Config::load_checked()?;
    config.saver.active = Some(canonical.clone());
    config
        .save_to_path(&toml_path)
        .map_err(|e| format!("failed to write {}: {e}", toml_path.display()))?;

    if !notified_daemon {
        println!("Active screensaver set to '{canonical}' (saved to config)");
    }

    Ok(())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn test_all_known_savers_contains_presets_and_all_53_scenes() {
        assert_eq!(ALL_KNOWN_SAVERS.len(), 56);
        assert!(ALL_KNOWN_SAVERS.contains(&"ascii"));
        assert!(ALL_KNOWN_SAVERS.contains(&"superfedora"));
        assert!(ALL_KNOWN_SAVERS.contains(&"tour"));
        for kind in EffectKind::ALL {
            assert!(
                ALL_KNOWN_SAVERS.contains(&kind.name()),
                "ALL_KNOWN_SAVERS missing {}",
                kind.name()
            );
        }
    }

    #[test]
    fn test_superfedora_unlock_gate_enforcement() {
        crate::ascii::sys_info::set_superfedora_override(Some(true));
        assert!(crate::ascii::sys_info::is_superfedora_machine());
        crate::ascii::sys_info::set_superfedora_override(None);
    }
}
