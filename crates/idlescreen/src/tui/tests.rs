// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Unit tests for settings TUI state, navigation, dirty-quit confirmation, and preview args.

use super::build_preview_args;
use super::input::{TuiAction, handle_key_event};
use super::model::{ModeChoice, TOTAL_FIELDS, TuiField, TuiModel};
use super::view::fit_to_width;
use crate::daemon::config::Config;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthStr;

#[test]
fn test_mode_choice_conversions() {
    assert_eq!(ModeChoice::from_config_str("os"), ModeChoice::HostOs);
    assert_eq!(ModeChoice::from_config_str("de"), ModeChoice::HostOs);
    assert_eq!(ModeChoice::from_config_str("kernel"), ModeChoice::HostOs);
    assert_eq!(
        ModeChoice::from_config_str("idlescreen"),
        ModeChoice::HostOs
    );
    assert_eq!(
        ModeChoice::from_config_str("custom"),
        ModeChoice::CustomText
    );

    assert_eq!(ModeChoice::HostOs.to_config_str(), "os");
    assert_eq!(ModeChoice::CustomText.to_config_str(), "custom");
}

#[test]
fn test_model_navigation_and_adjustment() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    assert_eq!(model.focused_index, 0);
    assert_eq!(model.current_field(), TuiField::ActiveSaver);

    for expected in 1..TOTAL_FIELDS {
        model.nav_next();
        assert_eq!(model.focused_index, expected);
    }
    model.nav_next();
    assert_eq!(model.focused_index, 0);
    model.nav_prev();
    assert_eq!(model.focused_index, TOTAL_FIELDS - 1);

    // Adjust timeout (index 4)
    model.focused_index = 4;
    assert_eq!(model.current_field(), TuiField::IdleTimeout);
    let old_timeout = model.idle_timeout_secs;
    model.adjust_right();
    assert_eq!(model.idle_timeout_secs, old_timeout + 30);
    assert!(model.dirty);
    model.adjust_left();
    assert_eq!(model.idle_timeout_secs, old_timeout);

    // Adjust dwell (index 5)
    model.focused_index = 5;
    assert_eq!(model.current_field(), TuiField::DwellDuration);
    let old_dwell = model.dwell_secs;
    model.adjust_right();
    assert!((model.dwell_secs - (old_dwell + 0.5)).abs() < f32::EPSILON);
    model.adjust_left();
    assert!((model.dwell_secs - old_dwell).abs() < f32::EPSILON);

    // Adjust FPS (index 6)
    model.focused_index = 6;
    assert_eq!(model.current_field(), TuiField::FrameRate);
    assert_eq!(model.fps, 0);
    model.adjust_right();
    assert_eq!(model.fps, 30);
    model.adjust_left();
    assert_eq!(model.fps, 0);

    // Adjust DPMS timeout (index 7)
    model.focused_index = 7;
    assert_eq!(model.current_field(), TuiField::DpmsTimeout);
    assert_eq!(model.dpms_timeout_secs, 0);
    model.adjust_right();
    assert_eq!(model.dpms_timeout_secs, 60);
    model.adjust_by(1, true);
    assert_eq!(model.dpms_timeout_secs, 360);
    model.adjust_by(-1, true);
    assert_eq!(model.dpms_timeout_secs, 60);
    model.adjust_left();
    assert_eq!(model.dpms_timeout_secs, 0);
}

#[test]
fn test_scene_cycle_and_custom_text_edit() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    let start_scene = model.active_saver_name().to_string();
    model.step_saver(1);
    assert_ne!(model.active_saver_name(), start_scene);
    model.step_saver(-1);
    assert_eq!(model.active_saver_name(), start_scene);

    // Entering and exiting custom text without typing does NOT mark dirty or change mode
    model.dirty = false;
    model.mode = ModeChoice::HostOs;
    model.focused_index = 3;
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    handle_key_event(&mut model, enter);
    assert!(model.editing_text);
    assert!(!model.dirty);
    assert_eq!(model.mode, ModeChoice::HostOs);

    handle_key_event(&mut model, enter);
    assert!(!model.editing_text);
    assert!(!model.dirty);
    assert_eq!(model.mode, ModeChoice::HostOs);

    // Typing a character switches mode to CustomText and marks dirty
    handle_key_event(&mut model, enter);
    model.insert_char('Z');
    assert_eq!(model.mode, ModeChoice::CustomText);
    assert!(model.dirty);
    handle_key_event(&mut model, enter);
    assert!(!model.editing_text);
}

#[test]
fn test_text_editing_and_utf8_cursor() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    model.custom_text.clear();
    model.cursor_pos = 0;
    model.editing_text = true;

    model.insert_char('A');
    model.insert_char('é');
    model.insert_char('你');
    assert_eq!(model.custom_text, "Aé你");
    assert_eq!(model.cursor_pos, 3);

    model.delete_back();
    assert_eq!(model.custom_text, "Aé");
    assert_eq!(model.cursor_pos, 2);

    let fitted = fit_to_width(&model.custom_text, model.cursor_pos, 20, true);
    assert_eq!(UnicodeWidthStr::width(fitted.as_str()), 20);
}

#[test]
fn test_ctrl_c_and_dirty_quit_confirmation() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    model.dirty = true;

    // Q when dirty triggers confirmation instead of immediate quit
    let q_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    assert!(matches!(
        handle_key_event(&mut model, q_event),
        TuiAction::None
    ));
    assert!(model.quit_confirm_pending);

    // Esc cancels confirmation, does not quit
    let esc_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(matches!(
        handle_key_event(&mut model, esc_event),
        TuiAction::None
    ));
    assert!(!model.quit_confirm_pending);

    // Ctrl+C quits immediately even when dirty or editing text
    model.editing_text = true;
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(matches!(
        handle_key_event(&mut model, ctrl_c),
        TuiAction::Quit
    ));
}

#[test]
fn test_preview_args_reflect_live_model_state() {
    let config = Config::default();
    let mut model = TuiModel::from_config(&config);
    model.mode = ModeChoice::CustomText;
    model.custom_text = "RETRO".to_string();
    model.dwell_secs = 4.5;

    let term_args = build_preview_args(&model, false);
    assert!(term_args.contains(&"--terminal".to_string()));
    assert!(term_args.contains(&"4.5".to_string()));
    assert!(term_args.contains(&"RETRO".to_string()));

    // Pressing P while focused on FullscreenButton (index 9) must trigger TerminalPreview
    model.focused_index = 9;
    let p_key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(
        handle_key_event(&mut model, p_key),
        TuiAction::TriggerTerminalPreview
    );

    // Pressing F while focused on PreviewButton (index 8) must trigger FullscreenPreview
    model.focused_index = 8;
    let f_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
    assert_eq!(
        handle_key_event(&mut model, f_key),
        TuiAction::TriggerFullscreenPreview
    );
}

#[test]
fn test_format_inhibitor_summary_extracts_app_names() {
    use super::model::{format_inhibitor_summary, format_saver_display};
    assert_eq!(format_inhibitor_summary("[]"), "Uninhibited");
    assert_eq!(
        format_inhibitor_summary(r#"[{"cookie":1,"app":"Firefox","reason":"Video"}]"#),
        "Inhibited (1: Firefox)"
    );
    assert_eq!(
        format_inhibitor_summary(
            r#"[{"cookie":1,"app":"Firefox","reason":"Video"},{"cookie":2,"app":"Steam","reason":"Game"}]"#
        ),
        "Inhibited (2: Firefox, Steam)"
    );
    assert_eq!(
        format_saver_display("tour"),
        "tour (All 50 Arcade Scenes in Order 1-50)"
    );

    let mut cfg = Config::default();
    cfg.saver.active = Some("new".to_string());
    let model = TuiModel::from_config(&cfg);
    assert_eq!(model.active_saver_name(), "tour");
}

#[test]
fn test_side_by_side_mini_preview_wide_vs_narrow() {
    use super::mini_preview::{MiniPreview, WIDE_TERM_MIN_COLS};
    use super::view::format_view_buffer;
    use std::time::Duration;

    let mut model = TuiModel::from_config(&Config::default());
    let mut preview = MiniPreview::new(&model, 125);
    let mut buf = String::new();

    // Narrow terminal (< 105 cols) renders single-pane layout without mini-preview box
    format_view_buffer(&model, Some(&preview), 80, &mut buf);
    assert!(!buf.contains("Live Preview"));
    assert!(buf.contains("[7] DPMS Power-Off:"));

    // Wide terminal (>= 105 cols) renders side-by-side mini-preview box and steps live
    format_view_buffer(&model, Some(&preview), WIDE_TERM_MIN_COLS, &mut buf);
    assert!(buf.contains("Live Preview"));

    model.step_saver(1);
    preview.sync_with_model(&model, 125);
    preview.step(Duration::from_millis(50));
    assert_eq!(preview.active_saver, model.active_saver_name());
}
