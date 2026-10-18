// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use std::time::{Duration, Instant};

use wayland_client::WEnum;
use wayland_client::protocol::{wl_keyboard, wl_pointer};

use super::{
    INPUT_DEBOUNCE_WINDOW, POINTER_DRIFT_THRESHOLD_PX, WaylandState, arcade_grid_dimensions,
};

fn past_debounce() -> Instant {
    Instant::now()
        .checked_sub(Duration::from_millis(600))
        .unwrap_or_else(Instant::now)
}

#[test]
fn test_input_debounce_constants() {
    assert_eq!(INPUT_DEBOUNCE_WINDOW, Duration::from_millis(500));
    assert!((POINTER_DRIFT_THRESHOLD_PX - 15.0).abs() < f64::EPSILON);
}

#[test]
fn test_key_debounce_ignores_release_and_early_press() {
    let mut state = WaylandState::new();

    state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Released));
    assert!(!state.exit_requested);

    state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Pressed));
    assert!(!state.exit_requested);

    state.start_time = past_debounce();
    state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Released));
    assert!(!state.exit_requested);

    state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Pressed));
    assert!(state.exit_requested);
}

#[test]
fn test_pointer_button_debounce() {
    let mut state = WaylandState::new();

    state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Released));
    assert!(!state.exit_requested);

    state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Pressed));
    assert!(!state.exit_requested);

    state.start_time = past_debounce();
    state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Released));
    assert!(!state.exit_requested);

    state.handle_pointer_button(WEnum::Value(wl_pointer::ButtonState::Pressed));
    assert!(state.exit_requested);
}

#[test]
fn test_pointer_motion_settling_and_drift() {
    let mut state = WaylandState::new();

    state.handle_pointer_motion(100.0, 100.0);
    assert_eq!(state.initial_pointer_pos, Some((100.0, 100.0)));
    assert!(!state.exit_requested);

    state.handle_pointer_motion(120.0, 120.0);
    assert_eq!(state.initial_pointer_pos, Some((120.0, 120.0)));
    assert!(!state.exit_requested);

    state.start_time = past_debounce();

    state.handle_pointer_motion(125.0, 128.0);
    assert!(!state.exit_requested);

    state.handle_pointer_motion(140.0, 120.0);
    assert!(state.exit_requested);
}

#[test]
fn test_pointer_enter_settles_position() {
    let mut state = WaylandState::new();

    state.handle_pointer_enter(50.0, 50.0);
    assert_eq!(state.initial_pointer_pos, Some((50.0, 50.0)));

    state.handle_pointer_enter(60.0, 60.0);
    assert_eq!(state.initial_pointer_pos, Some((60.0, 60.0)));
}

#[test]
fn test_pointer_axis_debounce() {
    let mut state = WaylandState::new();

    state.handle_pointer_axis();
    assert!(!state.exit_requested);

    state.start_time = past_debounce();
    state.handle_pointer_axis();
    assert!(state.exit_requested);
}

#[test]
fn test_pointer_motion_without_prior_enter() {
    let mut state = WaylandState::new();
    state.start_time = past_debounce();
    assert_eq!(state.initial_pointer_pos, None);

    state.handle_pointer_motion(200.0, 300.0);
    assert_eq!(state.initial_pointer_pos, Some((200.0, 300.0)));
    assert!(!state.exit_requested);

    state.handle_pointer_motion(205.0, 305.0);
    assert!(!state.exit_requested);

    state.handle_pointer_motion(220.0, 300.0);
    assert!(state.exit_requested);
}

#[test]
fn test_unknown_key_and_button_events_ignored() {
    let mut state = WaylandState::new();
    state.start_time = past_debounce();

    state.handle_key_event(WEnum::Unknown(99));
    assert!(!state.exit_requested);

    state.handle_pointer_button(WEnum::Unknown(99));
    assert!(!state.exit_requested);
}

#[test]
fn test_pointer_enter_after_debounce_establishes_position() {
    let mut state = WaylandState::new();
    state.start_time = past_debounce();

    state.handle_pointer_enter(50.0, 50.0);
    assert_eq!(state.initial_pointer_pos, Some((50.0, 50.0)));
    assert!(!state.exit_requested);

    state.handle_pointer_enter(100.0, 100.0);
    assert_eq!(state.initial_pointer_pos, Some((50.0, 50.0)));
    assert!(!state.exit_requested);
}

#[test]
fn test_subpixel_motion_and_negative_coords() {
    let mut state = WaylandState::new();
    state.handle_pointer_motion(-50.5, -30.25);
    assert_eq!(state.initial_pointer_pos, Some((-50.5, -30.25)));

    state.start_time = past_debounce();

    state.handle_pointer_motion(-40.0, -25.0);
    assert!(!state.exit_requested);

    state.handle_pointer_motion(-40.0, -10.0);
    assert!(state.exit_requested);
}

#[test]
fn test_arcade_grid_dimensions_aspect_ratios() {
    assert_eq!(arcade_grid_dimensions(1920, 1080), (120, 36));
    assert_eq!(arcade_grid_dimensions(3840, 2160), (120, 36));
    assert_eq!(arcade_grid_dimensions(1920, 1200), (120, 36));
    let (uw_cols, uw_rows) = arcade_grid_dimensions(3440, 1440);
    assert_eq!(uw_rows, 36);
    assert!((155..=165).contains(&uw_cols), "got {uw_cols}x{uw_rows}");
    assert_eq!(arcade_grid_dimensions(5120, 1440), (240, 36));
    let (p_cols, p_rows) = arcade_grid_dimensions(1080, 1920);
    assert_eq!(p_cols, 96);
    assert!((85..=96).contains(&p_rows), "got {p_cols}x{p_rows}");
}

#[test]
#[allow(unsafe_code)]
fn test_build_ascii_saver_b4_gating() {
    use super::frame_step::build_ascii_saver;
    // SAFETY: unit test sets and restores scoped environment variable.
    unsafe {
        std::env::set_var("IDLE_SUPERFEDORA", "1");
    }

    // 1. saver_name = "superfedora" without custom text -> uses SuperFedora stacked preset
    let sf_saver = build_ascii_saver(
        "superfedora",
        "superfedora",
        5.0,
        0.7,
        None,
        vec!["superfedora".into()],
        60,
    );
    assert_eq!(sf_saver.rotation.targets, vec!["superfedora"]);
    assert_eq!(sf_saver.rotation.active_text(), "SUPERFEDORA");

    // 2. saver_name = "ascii" with explicit targets ["de", "kernel"] -> must NOT be hijacked by SuperFedora lock (B4)
    let ascii_saver = build_ascii_saver(
        "ascii",
        "de,kernel",
        5.0,
        0.7,
        None,
        vec!["de".into(), "kernel".into()],
        60,
    );
    assert_eq!(ascii_saver.rotation.targets, vec!["de", "kernel"]);
    assert_ne!(ascii_saver.rotation.active_text(), "SUPERFEDORA");

    // 3. saver_name = "ascii" with custom text -> uses custom text
    let custom_saver = build_ascii_saver(
        "ascii",
        "custom",
        5.0,
        0.7,
        Some("MYBOX".into()),
        vec!["custom".into()],
        60,
    );
    assert_eq!(custom_saver.rotation.active_text(), "MYBOX");

    // SAFETY: unit test restores environment variable.
    unsafe {
        std::env::remove_var("IDLE_SUPERFEDORA");
    }
}

#[test]
fn test_input_wakes_dpms_and_subcell_teardown_damage() {
    use super::compute_cell_damage_rect;
    use crate::cell::TerminalCell;
    use crate::wayland::dpms::OutputPowerMode;
    let mut state = WaylandState::new();
    state.start_time = past_debounce();
    state.dpms.set_power_mode(OutputPowerMode::Off);
    assert!(state.dpms.is_sleeping());

    state.handle_key_event(WEnum::Value(wl_keyboard::KeyState::Pressed));
    assert!(state.exit_requested && !state.dpms.is_sleeping());

    let mut prev = vec![TerminalCell::default(); 8];
    prev[0].ch = '▀';
    let mut next = prev.clone();
    next[0].ch = '█';
    assert_eq!(
        compute_cell_damage_rect(&prev, &next, (4, 2), (32, 32), true),
        Some((0, 0, 32, 32))
    );
}
