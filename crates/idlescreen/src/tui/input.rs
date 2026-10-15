// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Keyboard event dispatch, text editing methods, and action processing for the settings TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::model::{ModeChoice, TuiField, TuiModel, char_to_byte_idx};
use super::save::save_settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiAction {
    None,
    TriggerTerminalPreview,
    TriggerFullscreenPreview,
    Quit,
}

impl TuiModel {
    fn mark_custom_edited(&mut self) {
        self.mode = ModeChoice::CustomText;
        self.dirty = true;
        self.quit_confirm_pending = false;
        self.update_status_sequence();
    }

    pub fn insert_char(&mut self, c: char) {
        let len = self.custom_text.chars().count();
        if len < 80 && !c.is_control() {
            let pos = self.cursor_pos.min(len);
            let b = char_to_byte_idx(&self.custom_text, pos);
            self.custom_text.insert(b, c);
            self.cursor_pos = pos + 1;
            self.mark_custom_edited();
        }
    }

    pub fn delete_back(&mut self) {
        let len = self.custom_text.chars().count();
        if self.cursor_pos > 0 && len > 0 {
            let pos = self.cursor_pos.min(len) - 1;
            let b = char_to_byte_idx(&self.custom_text, pos);
            self.custom_text.remove(b);
            self.cursor_pos = pos;
            self.mark_custom_edited();
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos < self.custom_text.chars().count() {
            let b = char_to_byte_idx(&self.custom_text, self.cursor_pos);
            self.custom_text.remove(b);
            self.mark_custom_edited();
        }
    }
}

pub fn handle_key_event(model: &mut TuiModel, key: KeyEvent) -> TuiAction {
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c' | 'C' | 'd' | 'D'))
    {
        return TuiAction::Quit;
    }
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
        }
        KeyCode::Backspace => model.delete_back(),
        KeyCode::Delete => model.delete_forward(),
        KeyCode::Left => model.adjust_left(),
        KeyCode::Right => model.adjust_right(),
        KeyCode::Home => model.cursor_pos = 0,
        KeyCode::End => model.cursor_pos = model.custom_text.chars().count(),
        KeyCode::Enter | KeyCode::Esc => {
            model.editing_text = false;
        }
        KeyCode::Up | KeyCode::BackTab => {
            model.editing_text = false;
            model.nav_prev();
        }
        KeyCode::Down | KeyCode::Tab => {
            model.editing_text = false;
            model.nav_next();
        }
        _ => {}
    }
    TuiAction::None
}

fn handle_normal_navigation(model: &mut TuiModel, key: KeyEvent) -> TuiAction {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => model.nav_prev(),
        KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => model.nav_next(),
        KeyCode::Left | KeyCode::Char('h') => model.adjust_by(-1, shift),
        KeyCode::Right | KeyCode::Char('l') => model.adjust_by(1, shift),
        KeyCode::PageUp | KeyCode::Char('[') => model.adjust_by(-1, true),
        KeyCode::PageDown | KeyCode::Char(']') => model.adjust_by(1, true),
        KeyCode::Char('1') => model.select_mode(ModeChoice::HostOs),
        KeyCode::Char('2') => model.select_mode(ModeChoice::CustomText),
        KeyCode::Char('s' | 'S') => execute_save(model),
        KeyCode::Char('t' | 'T') => {
            let active = crate::cli::service::toggle_service_quiet();
            model.refresh_daemon_status();
            model.status = if active {
                "Service enabled (Idle monitoring ACTIVE)".to_string()
            } else {
                "Service paused (Standby / Stay-Awake ON)".to_string()
            };
        }
        KeyCode::Char('p' | 'P') => return TuiAction::TriggerTerminalPreview,
        KeyCode::Char('f' | 'F') => return TuiAction::TriggerFullscreenPreview,
        KeyCode::Esc => {
            if model.quit_confirm_pending {
                model.quit_confirm_pending = false;
                model.status = "Quit cancelled".to_string();
            }
        }
        KeyCode::Char('q' | 'Q') => return request_quit(model),
        KeyCode::Char(' ') | KeyCode::Enter => match model.current_field() {
            TuiField::ActiveSaver => model.step_saver(1),
            TuiField::ModeChoice(idx) => {
                if let Some(&choice) = ModeChoice::ALL.get(idx) {
                    model.set_mode(choice);
                }
            }
            TuiField::CustomText => {
                model.editing_text = true;
                model.quit_confirm_pending = false;
            }
            TuiField::IdleTimeout | TuiField::DwellDuration | TuiField::FrameRate => {
                model.adjust_right();
            }
            TuiField::SaveButton => execute_save(model),
            TuiField::PreviewButton => return TuiAction::TriggerTerminalPreview,
            TuiField::FullscreenButton => return TuiAction::TriggerFullscreenPreview,
            TuiField::QuitButton => return request_quit(model),
        },
        _ => {}
    }
    TuiAction::None
}

fn request_quit(model: &mut TuiModel) -> TuiAction {
    if model.dirty && !model.quit_confirm_pending {
        model.quit_confirm_pending = true;
        model.status =
            "Unsaved changes — press Q again to quit without saving, or S to save".to_string();
        TuiAction::None
    } else {
        TuiAction::Quit
    }
}

fn execute_save(model: &mut TuiModel) {
    match save_settings(model) {
        Ok(msg) => {
            model.status = msg;
            model.dirty = false;
            model.quit_confirm_pending = false;
            model.refresh_daemon_status();
        }
        Err(err) => {
            model.status = format!("Error: {err}");
        }
    }
}
