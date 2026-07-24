// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Control subcommands: timeout, saver, preview, overlays, inhibitors.

use anyhow::{Context, Result, bail};
use idle_dbus::TranceClient;

pub fn cmd_timeout(client: &TranceClient, args: &[String]) -> Result<()> {
    let minutes = match args {
        [value] => value
            .parse::<u32>()
            .context("timeout requires a number of minutes (1–240)")?,
        _ => bail!("usage: idle timeout <minutes>"),
    };
    client.set_timeout(minutes).context("setting idle timeout")
}

pub fn cmd_saver(client: &TranceClient, args: &[String]) -> Result<()> {
    match args {
        [cmd, name] if cmd == "set" => {
            let dbus_name = if name == "random" { "" } else { name.as_str() };
            client
                .set_saver(dbus_name)
                .context("setting active saver via d-bus")
        }
        [cmd] if cmd == "list" => cmd_list(client),
        _ => bail!("usage: idle saver set <name|random> | trance saver list"),
    }
}

pub fn cmd_list(client: &TranceClient) -> Result<()> {
    let savers = client
        .list_savers()
        .context("listing installed savers via d-bus")?;
    for saver in savers {
        println!("{saver}");
    }
    Ok(())
}

pub fn cmd_inhibitors(client: &TranceClient) -> Result<()> {
    let inhibitors = client
        .list_inhibitors()
        .context("listing active inhibitors via d-bus")?;
    if inhibitors.is_empty() {
        println!("No active inhibitors.");
    } else {
        println!("Active inhibitors:");
        for (cookie, app, reason) in inhibitors {
            println!("  [{cookie}] {app}: {reason}");
        }
    }
    Ok(())
}

pub fn cmd_preview(client: &TranceClient, args: &[String]) -> Result<()> {
    let name = args.first().context("usage: idle preview <saver>")?;
    client.preview(name).context("starting preview via d-bus")
}

pub fn cmd_fps_overlay(client: &TranceClient, args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().context("querying daemon status")?;
            println!(
                "fps overlay: {}",
                if status.show_fps_overlay { "on" } else { "off" }
            );
            Ok(())
        }
        Some("on") => client
            .set_show_fps_overlay(true)
            .context("enabling fps overlay via d-bus"),
        Some("off") => client
            .set_show_fps_overlay(false)
            .context("disabling fps overlay via d-bus"),
        Some(value) => Err(anyhow::anyhow!(
            "unknown fps-overlay subcommand: {value} (use on, off, status)"
        )),
    }
}

fn parse_render_scale_value(value: &str) -> Result<f32> {
    let scale = value
        .parse::<f32>()
        .context("render-scale requires a number between 0.25 and 1.0")?;
    if !(0.25..=1.0).contains(&scale) {
        bail!("render-scale must be between 0.25 and 1.0");
    }
    Ok(scale)
}

pub fn cmd_render_scale(client: &TranceClient, args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().context("querying daemon status")?;
            println!(
                "render scale: {}",
                if status.render_scale.is_empty() {
                    "default"
                } else {
                    &status.render_scale
                }
            );
            Ok(())
        }
        Some("default") => client
            .set_render_scale(0.0)
            .context("resetting render scale via d-bus"),
        Some(value) => {
            let scale = parse_render_scale_value(value)?;
            client
                .set_render_scale(scale)
                .context("setting render scale via d-bus")
        }
    }
}
