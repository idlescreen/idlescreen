// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Keyboard event dispatch and action processing for the settings TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::model::{ModeChoice, TuiField, TuiModel};
use super::save::save_settings;

pub enum TuiAction {
    None,
    TriggerPreview,
    Quit,
}

pub fn handle_key_event(model: &mut TuiModel, key: KeyEvent) -> TuiAction {
    if model.editing_text {
        handle_text_editing(model, key)
    } else {
        handle_normal_navigation(model, key)
    }
}

fn handle_text_editing(model: &mut TuiModel, key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            model.insert_char(c);
            TuiAction::None
        }
        KeyCode::Backspace => {
            model.delete_back();
            TuiAction::None
        }
        KeyCode::Delete => {
            model.delete_forward();
            TuiAction::None
        }
        KeyCode::Left => {
            model.adjust_left();
            TuiAction::None
        }
        KeyCode::Right => {
            model.adjust_right();
            TuiAction::None
        }
        KeyCode::Home => {
            model.cursor_pos = 0;
            TuiAction::None
        }
        KeyCode::End => {
            model.cursor_pos = model.custom_text.len();
            TuiAction::None
        }
        KeyCode::Enter | KeyCode::Esc => {
            model.editing_text = false;
            model.mode = ModeChoice::CustomText;
            model.dirty = true;
            TuiAction::None
        }
        KeyCode::Up | KeyCode::BackTab => {
            model.editing_text = false;
            model.mode = ModeChoice::CustomText;
            model.nav_prev();
            TuiAction::None
        }
        KeyCode::Down | KeyCode::Tab => {
            model.editing_text = false;
            model.mode = ModeChoice::CustomText;
            model.nav_next();
            TuiAction::None
        }
        _ => TuiAction::None,
    }
}

fn handle_normal_navigation(model: &mut TuiModel, key: KeyEvent) -> TuiAction {
    match key.code {
        KeyCode::Up | KeyCode::BackTab => {
            model.nav_prev();
            TuiAction::None
        }
        KeyCode::Down | KeyCode::Tab => {
            model.nav_next();
            TuiAction::None
        }
        KeyCode::Left => {
            model.adjust_left();
            TuiAction::None
        }
        KeyCode::Right => {
            model.adjust_right();
            TuiAction::None
        }
        KeyCode::Char('1') => {
            model.select_mode(ModeChoice::HostOs);
            TuiAction::None
        }
        KeyCode::Char('2') => {
            model.select_mode(ModeChoice::DesktopEnv);
            TuiAction::None
        }
        KeyCode::Char('3') => {
            model.select_mode(ModeChoice::Kernel);
            TuiAction::None
        }
        KeyCode::Char('4') => {
            model.select_mode(ModeChoice::All3Cycle);
            TuiAction::None
        }
        KeyCode::Char('5') => {
            model.select_mode(ModeChoice::CustomText);
            TuiAction::None
        }
        KeyCode::Char('s' | 'S') => {
            execute_save(model);
            TuiAction::None
        }
        KeyCode::Char('p' | 'P') => TuiAction::TriggerPreview,
        KeyCode::Char('q' | 'Q') | KeyCode::Esc => TuiAction::Quit,
        KeyCode::Char(' ') | KeyCode::Enter => match model.current_field() {
            TuiField::ModeChoice(idx) => {
                model.mode = ModeChoice::ALL[idx];
                model.dirty = true;
                TuiAction::None
            }
            TuiField::CustomText => {
                model.editing_text = true;
                model.mode = ModeChoice::CustomText;
                model.dirty = true;
                TuiAction::None
            }
            TuiField::DwellDuration => {
                model.adjust_right();
                TuiAction::None
            }
            TuiField::IdleTimeout => {
                model.adjust_right();
                TuiAction::None
            }
            TuiField::SaveButton => {
                execute_save(model);
                TuiAction::None
            }
            TuiField::PreviewButton => TuiAction::TriggerPreview,
            TuiField::QuitButton => TuiAction::Quit,
        },
        _ => TuiAction::None,
    }
}

fn execute_save(model: &mut TuiModel) {
    match save_settings(model) {
        Ok(msg) => {
            model.status = msg;
            model.dirty = false;
        }
        Err(err) => {
            model.status = format!("Error: {err}");
        }
    }
}
