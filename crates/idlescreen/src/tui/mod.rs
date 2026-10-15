// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Interactive terminal user interface for 16-bit arcade screensaver configuration.

pub mod input;
pub mod mini_preview;
pub mod mode_choice;
pub mod model;
pub mod save;
#[cfg(test)]
mod tests;
pub mod view;

use std::io::IsTerminal;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};

use crate::cli::preview::term_guard::TerminalGuard;
use crate::daemon::config::Config;
use input::{TuiAction, handle_key_event};
use mini_preview::{MiniPreview, WIDE_TERM_MIN_COLS};
use mode_choice::ModeChoice;
use model::TuiModel;
use view::{render_view_into, render_view_with_preview_into};

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
    let mut frame_buf = String::with_capacity(8192);

    let is_interactive = std::io::stdin().is_terminal();
    if !is_interactive {
        render_view_into(&model, &mut frame_buf)?;
        return Ok(());
    }

    let mut guard = TerminalGuard::enter();
    let mut term_cols = crossterm::terminal::size().map_or(80, |(c, _)| c as usize);
    let mut mini_preview = MiniPreview::new(&model, term_cols);
    render_view_with_preview_into(&model, Some(&mini_preview), term_cols, &mut frame_buf)?;
    let mut last_tick = Instant::now();

    loop {
        if model.should_quit {
            break;
        }

        let poll_dur = if term_cols >= WIDE_TERM_MIN_COLS {
            if mini_preview.is_settled() {
                Duration::from_millis(90)
            } else {
                Duration::from_millis(50)
            }
        } else {
            Duration::from_millis(500)
        };

        if event::poll(poll_dur).unwrap_or(false) {
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
                            term_cols = crossterm::terminal::size().map_or(80, |(c, _)| c as usize);
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
                    mini_preview.sync_with_model(&model, term_cols);
                    last_tick = Instant::now();
                    render_view_with_preview_into(
                        &model,
                        Some(&mini_preview),
                        term_cols,
                        &mut frame_buf,
                    )?;
                }
                Ok(Event::Resize(w, _)) => {
                    term_cols = w as usize;
                    mini_preview.sync_with_model(&model, term_cols);
                    render_view_with_preview_into(
                        &model,
                        Some(&mini_preview),
                        term_cols,
                        &mut frame_buf,
                    )?;
                }
                Err(_) => break,
                _ => {}
            }
        } else if term_cols >= WIDE_TERM_MIN_COLS {
            let now = Instant::now();
            let dt = now.duration_since(last_tick);
            last_tick = now;
            mini_preview.step(dt);
            render_view_with_preview_into(&model, Some(&mini_preview), term_cols, &mut frame_buf)?;
        }
    }

    Ok(())
}
