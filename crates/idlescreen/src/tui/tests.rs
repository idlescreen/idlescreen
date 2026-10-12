// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Unit tests for settings TUI state, checklist toggling, and navigation logic.

use super::input::{TuiAction, handle_key_event};
use super::model::{ModeChoice, TuiModel};
use crate::daemon::config::Config;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_mode_choice_conversions() {
    assert_eq!(ModeChoice::from_config_str("os"), ModeChoice::HostOs);
    assert_eq!(ModeChoice::from_config_str("de"), ModeChoice::DesktopEnv);
    assert_eq!(ModeChoice::from_config_str("kernel"), ModeChoice::Kernel);
    assert_eq!(ModeChoice::from_config_str("idlescreen"), ModeChoice::Wordmark);
    assert_eq!(
        ModeChoice::from_config_str("custom"),
        ModeChoice::CustomText
    );

    assert_eq!(ModeChoice::HostOs.to_config_str(), "os");
    assert_eq!(ModeChoice::DesktopEnv.to_config_str(), "de");
    assert_eq!(ModeChoice::Kernel.to_config_str(), "kernel");
    assert_eq!(ModeChoice::Wordmark.to_config_str(), "idlescreen");
    assert_eq!(ModeChoice::CustomText.to_config_str(), "custom");
}

#[test]
fn test_model_navigation_and_adjustment() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    assert_eq!(model.focused_index, 0);

    model.nav_next();
    assert_eq!(model.focused_index, 1);

    model.nav_next();
    assert_eq!(model.focused_index, 2);

    model.nav_prev();
    assert_eq!(model.focused_index, 1);

    // Adjust timeout
    model.focused_index = 6;
    let old_timeout = model.idle_timeout_secs;
    model.adjust_right();
    assert_eq!(model.idle_timeout_secs, old_timeout + 30);
    model.adjust_left();
    assert_eq!(model.idle_timeout_secs, old_timeout);
}

#[test]
fn test_checklist_toggle_and_at_least_one_constraint() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    // Initially host OS (0) and DE (1) are checked
    assert!(model.target_checked[0]);
    assert!(model.target_checked[1]);
    assert!(!model.target_checked[2]);

    // Uncheck DE (1) -> succeeds, leaving Host OS (0)
    model.toggle_target(1);
    assert!(!model.target_checked[1]);
    assert!(model.target_checked[0]);

    // Try to uncheck Host OS (0) -> should be prevented by at-least-one constraint
    model.toggle_target(0);
    assert!(model.target_checked[0], "Cannot uncheck the only remaining selected target");

    // Toggle Kernel (2) -> now checked
    model.toggle_target(2);
    assert!(model.target_checked[2]);
    let active = model.active_targets();
    assert_eq!(active, vec!["os", "kernel"]);
}

#[test]
fn test_checklist_sequence_summary() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    model.detected_os = "FEDORA".to_string();
    model.detected_de = "GNOME".to_string();

    model.target_checked = [true, true, false, false, false];
    assert_eq!(model.sequence_summary(), "FEDORA -> GNOME -> FEDORA");

    model.target_checked = [true, false, false, false, false];
    assert_eq!(model.sequence_summary(), "Single (FEDORA)");
}

#[test]
fn test_text_editing_and_cursor() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    model.custom_text.clear();
    model.cursor_pos = 0;
    model.editing_text = true;

    model.insert_char('A');
    model.insert_char('B');
    assert_eq!(model.custom_text, "AB");
    assert_eq!(model.cursor_pos, 2);

    model.delete_back();
    assert_eq!(model.custom_text, "A");
    assert_eq!(model.cursor_pos, 1);
}

#[test]
fn test_key_actions() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);

    let q_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(matches!(
        handle_key_event(&mut model, q_event),
        TuiAction::Quit
    ));

    let p_event = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
    assert!(matches!(
        handle_key_event(&mut model, p_event),
        TuiAction::TriggerPreview
    ));
}

#[test]
fn test_numeric_mode_selection() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);

    let k1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
    handle_key_event(&mut model, k1);
    assert_eq!(model.focused_index, 0);

    let k2 = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
    handle_key_event(&mut model, k2);
    assert_eq!(model.focused_index, 1);

    let k3 = KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE);
    handle_key_event(&mut model, k3);
    assert_eq!(model.focused_index, 2);

    let k4 = KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE);
    handle_key_event(&mut model, k4);
    assert_eq!(model.focused_index, 3);

    let k5 = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
    handle_key_event(&mut model, k5);
    assert_eq!(model.focused_index, 4);
}

#[test]
fn test_custom_text_auto_switches_mode() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);

    // Nav to custom text box and press Enter to start editing
    model.focused_index = 5;
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    handle_key_event(&mut model, enter);
    assert!(model.editing_text);
    assert_eq!(model.mode, ModeChoice::CustomText);

    // Typing a character keeps CustomText and marks custom_text checked
    model.insert_char('Z');
    assert_eq!(model.mode, ModeChoice::CustomText);
    assert!(model.target_checked[4]);

    // Pressing Enter finishes editing and retains CustomText
    handle_key_event(&mut model, enter);
    assert!(!model.editing_text);
    assert_eq!(model.mode, ModeChoice::CustomText);
}

#[test]
fn test_fit_to_width_boundary() {
    use super::view::fit_to_width;

    let s = "Hello World";
    let fitted = fit_to_width(s, 5, 20);
    assert_eq!(fitted.chars().count(), 20);

    let long = "A".repeat(100);
    let fitted_long = fit_to_width(&long, 50, 60);
    assert_eq!(fitted_long.chars().count(), 60);
}
