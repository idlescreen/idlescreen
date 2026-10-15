// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Interactive terminal user interface for 16-bit arcade screensaver configuration.

pub mod input;
pub mod mode_choice;
pub mod model;
pub mod save;
#[cfg(test)]
mod tests;
pub mod view;

use std::io::IsTerminal;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

use crate::cli::preview::term_guard::TerminalGuard;
use crate::daemon::config::Config;
use input::{TuiAction, handle_key_event};
use mode_choice::ModeChoice;
use model::TuiModel;
use view::render_view_into;

pub fn build_preview_args(model: &TuiModel, fullscreen: bool) -> Vec<String> {
    let mut args = vec![
        model.active_saver_name().to_string(),
        if fullscreen {
            "--fullscreen".to_string()
        } else {
            "--terminal".to_string()
        },
        "--mode".to_string(),
        model.selected_targets().join(","),
        "--dwell".to_string(),
        model.dwell_secs.to_string(),
    ];
    if model.mode == ModeChoice::CustomText && !model.custom_text.trim().is_empty() {
        args.push("--text".to_string());
        args.push(model.custom_text.clone());
    }
    args
}

pub fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_or_migrate();
    let mut model = TuiModel::from_config(&config);
    let mut frame_buf = String::with_capacity(4096);

    let is_interactive = std::io::stdin().is_terminal();
    if !is_interactive {
        render_view_into(&model, &mut frame_buf)?;
        return Ok(());
    }

    let mut guard = TerminalGuard::enter();
    render_view_into(&model, &mut frame_buf)?;

    loop {
        if model.should_quit {
            break;
        }

        match event::read() {
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                match handle_key_event(&mut model, key) {
                    TuiAction::Quit => break,
                    action @ (TuiAction::TriggerTerminalPreview
                    | TuiAction::TriggerFullscreenPreview) => {
                        let fullscreen = action == TuiAction::TriggerFullscreenPreview;
                        let preview_args = build_preview_args(&model, fullscreen);
                        drop(guard);
                        let res = crate::cli::preview::execute(&preview_args);
                        guard = TerminalGuard::enter();
                        while event::poll(Duration::from_millis(0)).unwrap_or(false) {
                            let _ = event::read();
                        }
                        model.refresh_daemon_status();
                        model.status = match res {
                            Ok(()) => format!(
                                "Preview finished ({}) — tip: use ←/→ in preview to step scenes",
                                model.active_saver_name()
                            ),
                            Err(e) => format!("Preview error: {e}"),
                        };
                    }
                    TuiAction::None => {}
                }
                render_view_into(&model, &mut frame_buf)?;
            }
            Ok(Event::Resize(_, _)) => {
                render_view_into(&model, &mut frame_buf)?;
            }
            Err(_) => break,
            _ => {}
        }
    }

    Ok(())
}
