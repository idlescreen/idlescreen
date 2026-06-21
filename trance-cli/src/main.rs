// SPDX-License-Identifier: MIT

use std::process::ExitCode;

use trance_dbus::{daemon_available, TranceClient};

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("trance: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "help") {
        print_usage();
        return Ok(());
    }

    let client = TranceClient::connect().map_err(|error| {
        if daemon_available() {
            format!("failed to connect to daemon: {error}")
        } else {
            "trance-daemon is not running; start it with: systemctl --user start trance-daemon"
                .into()
        }
    })?;

    match args[0].as_str() {
        "status" => cmd_status(&client),
        "enable" => client.enable().map_err(map_dbus),
        "disable" => client.disable().map_err(map_dbus),
        "timeout" => cmd_timeout(&client, &args[1..]),
        "saver" => cmd_saver(&client, &args[1..]),
        "list" => cmd_list(&client),
        "preview" => cmd_preview(&client, &args[1..]),
        "stop" => client.stop_preview().map_err(map_dbus),
        "gpu" => cmd_gpu(&client, &args[1..]),
        "fps-overlay" => cmd_fps_overlay(&client, &args[1..]),
        "display-mode" => cmd_display_mode(&client, &args[1..]),
        "render-scale" => cmd_render_scale(&client, &args[1..]),
        _ => {
            print_usage();
            Err(format!("unknown command: {}", args[0]))
        }
    }
}

fn cmd_status(client: &TranceClient) -> Result<(), String> {
    let status = client.get_status().map_err(map_dbus)?;
    println!("running:              {}", status.running);
    println!("idle_enabled:         {}", status.idle_enabled);
    println!("idle_timeout_mins:    {}", status.idle_timeout_mins);
    println!("active_saver:         {}", display_saver(&status.active_saver));
    println!("gpu_enabled:          {}", status.gpu_enabled);
    println!("show_fps_overlay:     {}", status.show_fps_overlay);
    println!("display_mode:         {}", status.display_mode);
    println!(
        "render_scale:         {}",
        if status.render_scale.is_empty() {
            "default"
        } else {
            &status.render_scale
        }
    );
    println!("presentation_active:  {}", status.presentation_active);
    println!("preview_active:       {}", status.preview_active);
    println!("current_saver:        {}", status.current_saver);
    println!("system_idle:          {}", status.system_idle);
    println!("session_locked:       {}", status.session_locked);
    println!("inhibited:            {}", status.inhibited);
    Ok(())
}

fn cmd_timeout(client: &TranceClient, args: &[String]) -> Result<(), String> {
    let minutes = match args {
        [value] => value
            .parse::<u32>()
            .map_err(|_| "timeout requires a number of minutes (1–240)".to_string())?,
        _ => return Err("usage: trance timeout <minutes>".into()),
    };
    client.set_timeout(minutes).map_err(map_dbus)
}

fn cmd_saver(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args {
        [cmd, name] if cmd == "set" => {
            let dbus_name = if name == "random" { "" } else { name.as_str() };
            client.set_saver(dbus_name).map_err(map_dbus)
        }
        [cmd] if cmd == "list" => cmd_list(client),
        _ => Err("usage: trance saver set <name|random> | trance saver list".into()),
    }
}

fn cmd_list(client: &TranceClient) -> Result<(), String> {
    let savers = client.list_savers().map_err(map_dbus)?;
    for saver in savers {
        println!("{saver}");
    }
    Ok(())
}

fn cmd_preview(client: &TranceClient, args: &[String]) -> Result<(), String> {
    let name = args
        .first()
        .ok_or_else(|| "usage: trance preview <saver>".to_string())?;
    client.preview(name).map_err(map_dbus)
}

fn cmd_fps_overlay(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().map_err(map_dbus)?;
            println!(
                "fps overlay: {}",
                if status.show_fps_overlay { "on" } else { "off" }
            );
            Ok(())
        }
        Some("on") => client.set_show_fps_overlay(true).map_err(map_dbus),
        Some("off") => client.set_show_fps_overlay(false).map_err(map_dbus),
        Some(value) => Err(format!(
            "unknown fps-overlay subcommand: {value} (use on, off, status)"
        )),
    }
}

fn cmd_display_mode(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().map_err(map_dbus)?;
            println!("display mode: {}", status.display_mode);
            Ok(())
        }
        Some("primary") => client.set_display_mode("primary").map_err(map_dbus),
        Some("mirror") => client.set_display_mode("mirror").map_err(map_dbus),
        Some("expand") => client.set_display_mode("expand").map_err(map_dbus),
        Some(value) => Err(format!(
            "unknown display-mode: {value} (use primary, mirror, expand, status)"
        )),
    }
}

fn cmd_render_scale(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().map_err(map_dbus)?;
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
        Some("default") => client.set_render_scale(0.0).map_err(map_dbus),
        Some(value) => {
            let scale = value
                .parse::<f32>()
                .map_err(|_| "render-scale requires a number between 0.25 and 1.0".to_string())?;
            if !(0.25..=1.0).contains(&scale) {
                return Err("render-scale must be between 0.25 and 1.0".into());
            }
            client.set_render_scale(scale).map_err(map_dbus)
        }
    }
}

fn cmd_gpu(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().map_err(map_dbus)?;
            println!(
                "gpu rendering: {}",
                if status.gpu_enabled { "on" } else { "off" }
            );
            Ok(())
        }
        Some("on") => client.set_gpu_enabled(true).map_err(map_dbus),
        Some("off") => client.set_gpu_enabled(false).map_err(map_dbus),
        Some(value) => Err(format!("unknown gpu subcommand: {value} (use on, off, status)")),
    }
}

fn display_saver(name: &str) -> String {
    if name.is_empty() {
        "random".into()
    } else {
        name.to_string()
    }
}

fn map_dbus(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn print_usage() {
    eprintln!(
        "Usage: trance <command> [args]\n\
         \n\
         Commands:\n\
           status                 Show daemon state\n\
           enable | disable       Toggle idle screensaver\n\
           timeout <minutes>      Set idle timeout (1–240)\n\
           saver set <name|random>\n\
           saver list | list      List installed savers\n\
           preview <saver>        Preview a screensaver now\n\
           stop                   Stop preview or idle presentation\n\
           gpu on | off | status  Toggle GPU upscaling\n\
           fps-overlay on|off|status  Toggle on-screen FPS overlay\n\
           display-mode primary|mirror|expand|status  Multi-monitor layout\n\
           render-scale <0.25-1.0>|default|status  Simulation grid density (zoom)\n"
    );
}