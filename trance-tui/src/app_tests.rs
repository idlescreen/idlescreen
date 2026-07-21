// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

use super::*;

#[test]
fn test_app_initial_state() {
    let app = App::new();
    assert_eq!(app.active_pane, ActivePane::Settings);
    assert_eq!(app.selected_setting_idx, 0);
    assert_eq!(app.selected_saver_idx, 0);
    assert!(app.idle_timeout_mins >= 1 && app.idle_timeout_mins <= 240);
    assert!(app.render_scale >= 0.25 && app.render_scale <= 1.0);
}

#[test]
fn test_adjust_timeout_clamping() {
    let mut app = App::new();
    app.idle_timeout_mins = 10;
    app.adjust_timeout(-5);
    assert_eq!(app.idle_timeout_mins, 5);

    // Test minimum lower bound clamping (1 min)
    app.adjust_timeout(-100);
    assert_eq!(app.idle_timeout_mins, 1);

    // Test maximum upper bound clamping (240 min)
    app.adjust_timeout(500);
    assert_eq!(app.idle_timeout_mins, 240);
}

#[test]
fn test_adjust_scale_clamping() {
    let mut app = App::new();
    app.render_scale = 0.5;
    app.adjust_scale(0.2);
    assert!((app.render_scale - 0.7).abs() < 0.001);

    // Test lower bound clamping (0.25)
    app.adjust_scale(-2.0);
    assert_eq!(app.render_scale, 0.25);

    // Test upper bound clamping (1.0)
    app.adjust_scale(5.0);
    assert_eq!(app.render_scale, 1.0);
}

#[test]
fn test_active_pane_toggle() {
    let mut app = App::new();
    assert_eq!(app.active_pane, ActivePane::Settings);

    app.active_pane = match app.active_pane {
        ActivePane::Settings => ActivePane::Screensavers,
        ActivePane::Screensavers => ActivePane::Settings,
    };
    assert_eq!(app.active_pane, ActivePane::Screensavers);

    app.active_pane = match app.active_pane {
        ActivePane::Settings => ActivePane::Screensavers,
        ActivePane::Screensavers => ActivePane::Settings,
    };
    assert_eq!(app.active_pane, ActivePane::Settings);
}
