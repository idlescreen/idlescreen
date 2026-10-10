// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Unit tests for terminal screensaver preview execution.

use super::render::execute;

#[test]
fn test_headless_preview_completes_cleanly() {
    let args = vec![
        "ascii".to_string(),
        "--timeout".to_string(),
        "0.05".to_string(),
    ];
    let res = execute(&args);
    assert!(res.is_ok());
}

#[test]
fn test_headless_preview_with_mode_and_dwell() {
    let args = vec![
        "--mode".to_string(),
        "kernel".to_string(),
        "--dwell".to_string(),
        "1.0".to_string(),
        "--timeout".to_string(),
        "0.05".to_string(),
    ];
    let res = execute(&args);
    assert!(res.is_ok());
}
