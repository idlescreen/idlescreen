// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal rendering layout and view formatting for the settings TUI.

use std::fmt::Write as FmtWrite;
use std::io::{Write, stdout};

use crossterm::cursor::MoveTo;
use crossterm::execute;

use super::model::{ModeChoice, TuiField, TuiModel};

pub fn render_view(model: &TuiModel) -> std::io::Result<()> {
    let mut out = stdout();
    execute!(out, MoveTo(0, 0))?;

    let mut b = String::with_capacity(4096);
    let f = model.current_field();

    // 1. Header Box
    b.push_str(
        "\x1b[1;36m┌──────────────────────────────────────────────────────────────────┐\x1b[0m\r\n",
    );
    b.push_str(
        "\x1b[1;36m│             IdleScreen Settings — ASCII Screensaver              │\x1b[0m\r\n",
    );
    b.push_str("\x1b[1;36m└──────────────────────────────────────────────────────────────────┘\x1b[0m\r\n\r\n");

    // 2. Target Selection Checklist
    b.push_str("\x1b[1m[1] ASCII Text Selection (Space to toggle):\x1b[0m\r\n");
    for (i, &m) in ModeChoice::ALL.iter().enumerate() {
        let is_checked = model.target_checked[i];
        let is_focused = f == TuiField::ModeChoice(i);
        let check_box = if is_checked { "[x]" } else { "[ ]" };
        let pointer = if is_focused {
            " \x1b[1;33m▸\x1b[0m"
        } else {
            "  "
        };

        let sample = match m {
            ModeChoice::HostOs => &model.detected_os,
            ModeChoice::DesktopEnv => &model.detected_de,
            ModeChoice::Kernel => &model.detected_kernel,
            ModeChoice::Wordmark => "IDLESCREEN",
            ModeChoice::CustomText => {
                if model.custom_text.trim().is_empty() {
                    "(Custom text below)"
                } else {
                    &model.custom_text
                }
            }
        };

        let sample_str = if sample.chars().count() > 28 {
            let mut s: String = sample.chars().take(26).collect();
            s.push_str("..");
            s
        } else {
            sample.to_string()
        };

        if is_focused {
            let _ = writeln!(
                b,
                "{pointer} \x1b[7m {check_box} {:<24} [{:<28}] \x1b[0m\r",
                m.label(),
                sample_str
            );
        } else if is_checked {
            let _ = writeln!(
                b,
                "{pointer} \x1b[1;32m{check_box} {:<24} [{:<28}]\x1b[0m\r",
                m.label(),
                sample_str
            );
        } else {
            let _ = writeln!(
                b,
                "{pointer} \x1b[2m{check_box}\x1b[0m {:<24} \x1b[2m[{:<28}]\x1b[0m\r",
                m.label(),
                sample_str
            );
        }
    }
    let seq = model.sequence_summary();
    let _ = writeln!(
        b,
        "    \x1b[1;36m↳ Scene Cycle:\x1b[0m \x1b[1m{seq}\x1b[0m\r\n"
    );

    // 3. Custom Text Input Box
    let is_text_focused = f == TuiField::CustomText;
    let text_header_style = if is_text_focused {
        "\x1b[1;33m"
    } else {
        "\x1b[1m"
    };
    let _ = writeln!(
        b,
        "{text_header_style}[2] Custom Banner Text:\x1b[0m (Enter to edit, Esc/Enter to finish)\r"
    );

    let box_border = if is_text_focused {
        "\x1b[1;33m"
    } else {
        "\x1b[2m"
    };
    let _ = writeln!(
        b,
        "  {box_border}┌──────────────────────────────────────────────────────────────┐\x1b[0m\r"
    );

    // Render text with cursor clamped to exactly 60 visible columns
    let mut disp_text = model.custom_text.clone();
    if model.editing_text {
        let pos = model.cursor_pos.min(disp_text.chars().count());
        let byte_idx = super::model::char_to_byte_idx(&disp_text, pos);
        disp_text.insert(byte_idx, '│');
    } else if disp_text.is_empty() {
        disp_text = "(Press Enter to type custom banner)".to_string();
    }
    let visible_text = fit_to_width(&disp_text, model.cursor_pos, 60);

    if is_text_focused && model.editing_text {
        let _ = writeln!(
            b,
            "  {box_border}│\x1b[0m \x1b[1m{visible_text}\x1b[0m {box_border}│\x1b[0m\r"
        );
    } else if is_text_focused {
        let _ = writeln!(
            b,
            "  {box_border}│\x1b[0m \x1b[7m{visible_text}\x1b[0m {box_border}│\x1b[0m\r"
        );
    } else {
        let _ = writeln!(
            b,
            "  {box_border}│\x1b[0m {visible_text} {box_border}│\x1b[0m\r"
        );
    }
    let _ = writeln!(
        b,
        "  {box_border}└──────────────────────────────────────────────────────────────┘\x1b[0m\r\n"
    );

    // 4. Idle Timeout
    let is_to_focused = f == TuiField::IdleTimeout;
    let to_style = if is_to_focused {
        "\x1b[1;33m"
    } else {
        "\x1b[1m"
    };
    let _ = writeln!(
        b,
        "{to_style}[3] Idle Timeout (inactivity before screensaver activates):\x1b[0m\r"
    );
    let mins = model.idle_timeout_secs / 60;
    let secs = model.idle_timeout_secs % 60;
    let to_val = format!("{} s ({mins}m {secs:02}s)", model.idle_timeout_secs);
    if is_to_focused {
        let _ = writeln!(
            b,
            "   \x1b[7m [ ◄   {to_val:^16}   ► ] \x1b[0m  \x1b[2m(←/→ to adjust by 30s)\x1b[0m\r\n"
        );
    } else {
        let _ = writeln!(
            b,
            "    [ ◄   {to_val:^16}   ► ]   \x1b[2m(←/→ to adjust by 30s)\x1b[0m\r\n"
        );
    }

    // 6. Action Buttons
    let save_btn = if f == TuiField::SaveButton {
        "\x1b[7;1;32m [ S: Save & Apply ] \x1b[0m"
    } else {
        "\x1b[1;32m [ S: Save & Apply ] \x1b[0m"
    };
    let prev_btn = if f == TuiField::PreviewButton {
        "\x1b[7;1;36m [ P: Live Preview ] \x1b[0m"
    } else {
        "\x1b[1;36m [ P: Live Preview ] \x1b[0m"
    };
    let quit_btn = if f == TuiField::QuitButton {
        "\x1b[7;1;31m [ Q: Quit ] \x1b[0m"
    } else {
        "\x1b[1;31m [ Q: Quit ] \x1b[0m"
    };
    let _ = writeln!(b, "  {save_btn}     {prev_btn}     {quit_btn}\r\n");

    // 7. Status & Hints
    let dirty_indicator = if model.dirty {
        "\x1b[1;33m[Modified]\x1b[0m "
    } else {
        ""
    };
    let _ = writeln!(b, "  Status: {dirty_indicator}{}\r", model.status);
    b.push_str("  \x1b[2mKeys: [Tab/↑↓] Navigate   [Space/Enter] Toggle   [←→] Adjust   [S] Save   [P] Preview   [Q] Quit\x1b[0m\r\n");

    let _ = write!(out, "{b}");
    let _ = out.flush();
    Ok(())
}

pub fn fit_to_width(text: &str, cursor_idx: usize, max_len: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_len {
        let pad = max_len - char_count;
        let mut res = text.to_string();
        for _ in 0..pad {
            res.push(' ');
        }
        return res;
    }
    let start = cursor_idx
        .saturating_sub(max_len / 2)
        .min(char_count - max_len);
    let mut windowed: String = text.chars().skip(start).take(max_len).collect();
    let pad = max_len.saturating_sub(windowed.chars().count());
    for _ in 0..pad {
        windowed.push(' ');
    }
    windowed
}
