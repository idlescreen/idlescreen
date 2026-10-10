// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Interactive terminal user interface for ASCII screensaver configuration.

pub mod input;
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
use model::TuiModel;
use view::render_view;

pub fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_or_migrate();
    let mut model = TuiModel::from_config(&config);

    let is_interactive = std::io::stdin().is_terminal();
    if !is_interactive {
        render_view(&model)?;
        return Ok(());
    }

    let mut guard = TerminalGuard::enter();
    render_view(&model)?;

    loop {
        if model.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if key.kind != KeyEventKind::Release {
                        match handle_key_event(&mut model, key) {
                            TuiAction::Quit => break,
                            TuiAction::TriggerPreview => {
                                drop(guard);
                                let preview_args = vec![
                                    "ascii".to_string(),
                                    "--mode".to_string(),
                                    model.mode.to_config_str().to_string(),
                                    "--dwell".to_string(),
                                    model.dwell_secs.to_string(),
                                    "--text".to_string(),
                                    model.custom_text.clone(),
                                    "--timeout".to_string(),
                                    "10".to_string(),
                                ];
                                let _ = crate::cli::preview::execute(&preview_args);
                                guard = TerminalGuard::enter();
                                while event::poll(Duration::from_millis(0)).unwrap_or(false) {
                                    let _ = event::read();
                                }
                                model.status = "Preview finished".to_string();
                            }
                            TuiAction::None => {}
                        }
                        render_view(&model)?;
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    render_view(&model)?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
