// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use anyhow::{Context, Result};
use trance_dbus::{DaemonStatus, TranceClient};

enum MenuAction {
    ToggleIdle,
    ToggleFps,
    SetSaver,
    SetTimeout,
    Preview,
    Stop,
    Quit,
}

pub fn run_interactive(client: &TranceClient) -> Result<()> {
    loop {
        let status = client
            .get_status()
            .context("querying daemon status via d-bus")?;
        print_status(&status);
        match prompt_main_menu()? {
            MenuAction::Quit => break,
            MenuAction::ToggleIdle => toggle_idle(client, &status)?,
            MenuAction::ToggleFps => toggle_fps(client, &status)?,
            MenuAction::SetSaver => {
                if let Some(saver) = prompt_saver_select(client)? {
                    client
                        .set_saver(&saver)
                        .context("setting active saver via d-bus")?;
                }
            }
            MenuAction::SetTimeout => {
                if let Some(mins) = prompt_timeout()? {
                    client
                        .set_timeout(mins)
                        .with_context(|| format!("setting idle timeout to {mins} minutes"))?;
                }
            }
            MenuAction::Preview => preview_saver(client)?,
            MenuAction::Stop => {
                client
                    .stop_preview()
                    .context("stopping preview via d-bus")?;
                println!("Presentation stopped.");
            }
        }
    }
    Ok(())
}

fn print_status(status: &DaemonStatus) {
    println!("\n==========================================");
    println!("Trance Interactive Control Panel");
    println!("==========================================");
    println!(
        " 1. Toggle Idle Activation (Current: {})",
        if status.idle_enabled {
            "ENABLED"
        } else {
            "DISABLED"
        }
    );
    println!(
        " 2. Set Idle Timeout       (Current: {} mins)",
        status.idle_timeout_mins
    );
    println!(
        " 3. Select Active Saver    (Current: {})",
        if status.active_saver.is_empty() {
            "random"
        } else {
            &status.active_saver
        }
    );
    println!(" 4. Preview a Screensaver");
    println!(
        " 5. Toggle FPS Overlay     (Current: {})",
        if status.show_fps_overlay { "ON" } else { "OFF" }
    );
    println!(" 6. Stop Current Preview / Presentation");
    println!(" 7. Exit");
}

fn prompt_main_menu() -> Result<MenuAction> {
    loop {
        print!("\nSelect an option (1-7): ");
        io::stdout().flush().context("flushing stdout")?;
        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .context("reading selection from stdin")?;
        match choice.trim() {
            "1" => return Ok(MenuAction::ToggleIdle),
            "2" => return Ok(MenuAction::SetTimeout),
            "3" => return Ok(MenuAction::SetSaver),
            "4" => return Ok(MenuAction::Preview),
            "5" => return Ok(MenuAction::ToggleFps),
            "6" => return Ok(MenuAction::Stop),
            "7" => return Ok(MenuAction::Quit),
            _ => println!("Invalid selection. Please enter a number 1-7."),
        }
    }
}

fn prompt_saver_select(client: &TranceClient) -> Result<Option<String>> {
    let savers = client
        .list_savers()
        .context("listing installed savers via d-bus")?;
    println!("\nAvailable Screensavers:");
    println!("  0. random (default)");
    for (i, s) in savers.iter().enumerate() {
        println!("  {}. {s}", i + 1);
    }
    print!("Select a saver (0-{}): ", savers.len());
    io::stdout().flush().context("flushing stdout")?;
    let mut idx_str = String::new();
    io::stdin()
        .read_line(&mut idx_str)
        .context("reading selection from stdin")?;
    if let Ok(idx) = idx_str.trim().parse::<usize>() {
        if idx == 0 {
            println!("Active screensaver set to: random");
            Ok(Some(String::new()))
        } else if idx <= savers.len() {
            let name = savers[idx - 1].clone();
            println!("Active screensaver set to: {name}");
            Ok(Some(name))
        } else {
            println!("Invalid choice.");
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn prompt_timeout() -> Result<Option<u32>> {
    print!("Enter new timeout (1-240 mins): ");
    io::stdout().flush().context("flushing stdout")?;
    let mut timeout_str = String::new();
    io::stdin()
        .read_line(&mut timeout_str)
        .context("reading timeout from stdin")?;
    if let Ok(mins) = timeout_str.trim().parse::<u32>() {
        if (1..=240).contains(&mins) {
            println!("Timeout updated to {mins} minutes.");
            Ok(Some(mins))
        } else {
            println!("Invalid range (must be 1-240).");
            Ok(None)
        }
    } else {
        println!("Invalid input.");
        Ok(None)
    }
}

fn toggle_idle(client: &TranceClient, status: &DaemonStatus) -> Result<()> {
    if status.idle_enabled {
        client
            .disable()
            .context("disabling idle screensaver via d-bus")?;
        println!("Disabled screensaver activation.");
    } else {
        client
            .enable()
            .context("enabling idle screensaver via d-bus")?;
        println!("Enabled screensaver activation.");
    }
    Ok(())
}

fn toggle_fps(client: &TranceClient, status: &DaemonStatus) -> Result<()> {
    let new_state = !status.show_fps_overlay;
    client
        .set_show_fps_overlay(new_state)
        .context("toggling fps overlay via d-bus")?;
    println!(
        "FPS overlay toggled to {}.",
        if new_state { "ON" } else { "OFF" }
    );
    Ok(())
}

fn preview_saver(client: &TranceClient) -> Result<()> {
    let savers = client
        .list_savers()
        .context("listing installed savers via d-bus")?;
    println!("\nChoose screensaver to preview:");
    for (i, s) in savers.iter().enumerate() {
        println!("  {}. {s}", i + 1);
    }
    print!("Select a screensaver (1-{}): ", savers.len());
    io::stdout().flush().context("flushing stdout")?;
    let mut idx_str = String::new();
    io::stdin()
        .read_line(&mut idx_str)
        .context("reading selection from stdin")?;
    if let Ok(idx) = idx_str.trim().parse::<usize>()
        && idx >= 1
        && idx <= savers.len()
    {
        let name = &savers[idx - 1];
        client
            .preview(name)
            .with_context(|| format!("starting preview of '{name}'"))?;
        println!("Starting preview of {name}...");
    } else {
        println!("Invalid choice.");
    }
    Ok(())
}
