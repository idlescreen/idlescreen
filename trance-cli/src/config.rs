// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow, bail};
use trance_dbus::TranceClient;

pub fn handle_config(client: &TranceClient, args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("list") => cmd_config_list(client),
        Some("get") => {
            let key = args.get(1).ok_or_else(|| anyhow!("missing key"))?;
            cmd_config_get(client, key)
        }
        Some("set") => {
            let key = args.get(1).ok_or_else(|| anyhow!("missing key"))?;
            let value = args.get(2).ok_or_else(|| anyhow!("missing value"))?;
            cmd_config_set(client, key, value)
        }
        _ => {
            print_config_usage();
            Ok(())
        }
    }
}

fn cmd_config_list(client: &TranceClient) -> Result<()> {
    let status = client
        .get_status()
        .context("querying daemon status via d-bus")?;
    println!("idle_enabled:      {}", status.idle_enabled);
    println!("idle_timeout_mins: {}", status.idle_timeout_mins);
    println!(
        "active_saver:      {}",
        if status.active_saver.is_empty() {
            "random"
        } else {
            &status.active_saver
        }
    );
    println!("gpu_enabled:       {}", status.gpu_enabled);
    println!("show_fps_overlay:  {}", status.show_fps_overlay);
    println!(
        "render_scale:      {}",
        if status.render_scale.is_empty() {
            "default"
        } else {
            &status.render_scale
        }
    );
    Ok(())
}

fn cmd_config_get(client: &TranceClient, key: &str) -> Result<()> {
    let status = client
        .get_status()
        .context("querying daemon status via d-bus")?;
    match key {
        "idle_enabled" | "enabled" => println!("{}", status.idle_enabled),
        "idle_timeout_mins" | "timeout" => println!("{}", status.idle_timeout_mins),
        "active_saver" | "saver" => println!(
            "{}",
            if status.active_saver.is_empty() {
                "random"
            } else {
                &status.active_saver
            }
        ),
        "gpu_enabled" | "gpu" => println!("{}", status.gpu_enabled),
        "show_fps_overlay" | "fps" => println!("{}", status.show_fps_overlay),
        "render_scale" | "scale" => println!(
            "{}",
            if status.render_scale.is_empty() {
                "default"
            } else {
                &status.render_scale
            }
        ),
        _ => return Err(anyhow!("unknown configuration key: {key}")),
    }
    Ok(())
}

fn cmd_config_set(client: &TranceClient, key: &str, val: &str) -> Result<()> {
    match key {
        "idle_enabled" | "enabled" => set_idle_enabled(client, val)?,
        "idle_timeout_mins" | "timeout" => set_idle_timeout(client, val)?,
        "active_saver" | "saver" => set_active_saver(client, val)?,
        "gpu_enabled" | "gpu" => set_gpu_enabled(client, val)?,
        "show_fps_overlay" | "fps" => set_fps_overlay(client, val)?,
        "render_scale" | "scale" => set_render_scale(client, val)?,
        _ => return Err(anyhow!("unknown configuration key: {key}")),
    }
    println!("Set config key '{key}' to '{val}' successfully.");
    Ok(())
}

fn set_idle_enabled(client: &TranceClient, val: &str) -> Result<()> {
    let b = val
        .parse::<bool>()
        .map_err(|_| anyhow!("value must be true or false"))?;
    if b { client.enable() } else { client.disable() }
        .context("toggling idle screensaver via d-bus")?;
    Ok(())
}

fn set_idle_timeout(client: &TranceClient, val: &str) -> Result<()> {
    let n = val
        .parse::<u32>()
        .map_err(|_| anyhow!("value must be an integer (1–240)"))?;
    if !(1..=240).contains(&n) {
        bail!("timeout must be between 1 and 240 minutes");
    }
    client
        .set_timeout(n)
        .with_context(|| format!("setting idle timeout to {n} minutes"))?;
    Ok(())
}

fn set_active_saver(client: &TranceClient, val: &str) -> Result<()> {
    let name = if val == "random" || val == "none" {
        ""
    } else {
        val
    };
    client
        .set_saver(name)
        .with_context(|| format!("setting active saver to '{name}'"))?;
    Ok(())
}

fn set_gpu_enabled(client: &TranceClient, val: &str) -> Result<()> {
    let b = val
        .parse::<bool>()
        .map_err(|_| anyhow!("value must be true or false"))?;
    client
        .set_gpu_enabled(b)
        .context("toggling gpu upscaler via d-bus")?;
    Ok(())
}

fn set_fps_overlay(client: &TranceClient, val: &str) -> Result<()> {
    let b = val
        .parse::<bool>()
        .map_err(|_| anyhow!("value must be true or false"))?;
    client
        .set_show_fps_overlay(b)
        .context("toggling fps overlay via d-bus")?;
    Ok(())
}

fn set_render_scale(client: &TranceClient, val: &str) -> Result<()> {
    let scale = if val == "default" {
        0.0f32
    } else {
        val.parse::<f32>()
            .map_err(|_| anyhow!("value must be between 0.25 and 1.0, or 'default'"))?
    };
    if scale != 0.0 && !(0.25..=1.0).contains(&scale) {
        bail!("scale must be between 0.25 and 1.0, or 'default'");
    }
    client
        .set_render_scale(scale)
        .with_context(|| format!("setting render scale to {scale}"))?;
    Ok(())
}

fn print_config_usage() {
    println!("usage: trance config get <key> | set <key> <val> | list");
}
