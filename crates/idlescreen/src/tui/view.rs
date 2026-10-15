// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Flicker-free terminal rendering layout with synchronized output and Unicode display width.

use std::fmt::Write as FmtWrite;
use std::io::{Write, stdout};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::model::{
    ModeChoice, TuiField, TuiModel, char_to_byte_idx, format_fps_display, format_saver_display,
};

pub fn render_view(model: &TuiModel) -> std::io::Result<()> {
    let mut buf = String::with_capacity(4096);
    render_view_into(model, &mut buf)
}

pub fn render_view_into(model: &TuiModel, b: &mut String) -> std::io::Result<()> {
    let mut out = stdout();
    b.clear();
    b.push_str("\x1b[?2026h\x1b[H");
    let f = model.current_field();

    b.push_str("\x1b[1;36m┌──────────────────────────────────────────────────────────────────────┐\x1b[0m\x1b[K\r\n");
    b.push_str("\x1b[1;36m│           IdleScreen Settings — 16-Bit Arcade Screensaver            │\x1b[0m\x1b[K\r\n");
    b.push_str("\x1b[1;36m└──────────────────────────────────────────────────────────────────────┘\x1b[0m\x1b[K\r\n");
    let d_col = if model.daemon_status.starts_with("Active") {
        "\x1b[1;32m"
    } else {
        "\x1b[1;33m"
    };
    let _ = writeln!(
        b,
        "  Daemon: {d_col}{}\x1b[0m   │   Inhibitors: \x1b[1m{}\x1b[0m   \x1b[2m([T] Standby)\x1b[0m\x1b[K\r\n\x1b[K\r",
        model.daemon_status, model.inhibitor_summary
    );

    let s_hdr = if f == TuiField::ActiveSaver {
        "\x1b[1;33m"
    } else {
        "\x1b[1m"
    };
    let _ = writeln!(
        b,
        "{s_hdr}[1] Active Screensaver / Arcade Scene ({}/{}):\x1b[0m\x1b[K\r",
        model.selected_saver_idx + 1,
        model.available_savers.len()
    );
    let saver_disp = format_saver_display(model.active_saver_name());
    if f == TuiField::ActiveSaver {
        let _ = writeln!(
            b,
            "   \x1b[7;1;36m [ ◄  {saver_disp:^44}  ► ] \x1b[0m  \x1b[2m(←/→ or [/])\x1b[0m\x1b[K\r\n\x1b[K\r"
        );
    } else {
        let _ = writeln!(
            b,
            "    [ ◄  \x1b[1;36m{saver_disp:^44}\x1b[0m  ► ]   \x1b[2m(←/→ or [/])\x1b[0m\x1b[K\r\n\x1b[K\r"
        );
    }

    b.push_str("\x1b[1m[2] Banner Text Mode (Space/Enter or 1-2 to select):\x1b[0m\x1b[K\r\n");
    for (i, &m) in ModeChoice::ALL.iter().enumerate() {
        let is_selected = model.mode == m;
        let is_focused = f == TuiField::ModeChoice(i);
        let radio = if is_selected { "(●)" } else { "( )" };
        let ptr = if is_focused {
            " \x1b[1;33m▸\x1b[0m"
        } else {
            "  "
        };
        let sample = match m {
            ModeChoice::HostOs => model.detected_os.as_str(),
            ModeChoice::CustomText if model.custom_text.trim().is_empty() => "(Custom text below)",
            ModeChoice::CustomText => model.custom_text.as_str(),
        };
        let sample_str = fit_to_width(sample, 0, 26, false);
        if is_focused {
            let _ = writeln!(
                b,
                "{ptr} \x1b[7m {radio} {:<24} [{sample_str}] \x1b[0m\x1b[K\r",
                m.label()
            );
        } else if is_selected {
            let _ = writeln!(
                b,
                "{ptr} \x1b[1;32m{radio} {:<24} [{sample_str}]\x1b[0m\x1b[K\r",
                m.label()
            );
        } else {
            let _ = writeln!(
                b,
                "{ptr} \x1b[2m{radio}\x1b[0m {:<24} \x1b[2m[{sample_str}]\x1b[0m\x1b[K\r",
                m.label()
            );
        }
    }
    b.push_str("\x1b[K\r\n");

    let is_tf = f == TuiField::CustomText;
    let tf_hdr = if is_tf { "\x1b[1;33m" } else { "\x1b[1m" };
    let bdr = if is_tf { "\x1b[1;33m" } else { "\x1b[2m" };
    let _ = writeln!(
        b,
        "{tf_hdr}[3] Custom Banner Text:\x1b[0m \x1b[2m(Enter to edit, Esc/Enter to finish)\x1b[0m\x1b[K\r"
    );
    let _ = writeln!(
        b,
        "  {bdr}┌──────────────────────────────────────────────────────────────┐\x1b[0m\x1b[K\r"
    );
    let mut disp = model.custom_text.clone();
    if model.editing_text {
        let pos = model.cursor_pos.min(disp.chars().count());
        disp.insert(char_to_byte_idx(&disp, pos), '│');
    } else if disp.is_empty() {
        disp = "(Press Enter to type custom banner)".to_string();
    }
    let vis = fit_to_width(&disp, model.cursor_pos, 60, model.editing_text);
    let inner_style = if is_tf && model.editing_text {
        "\x1b[1m"
    } else if is_tf {
        "\x1b[7m"
    } else {
        ""
    };
    let _ = writeln!(
        b,
        "  {bdr}│\x1b[0m {inner_style}{vis}\x1b[0m {bdr}│\x1b[0m\x1b[K\r"
    );
    let _ = writeln!(
        b,
        "  {bdr}└──────────────────────────────────────────────────────────────┘\x1b[0m\x1b[K\r\n\x1b[K\r"
    );

    render_slider_row(
        b,
        "[4] Idle Timeout:",
        &format!(
            "{} s ({}m {:02}s)",
            model.idle_timeout_secs,
            model.idle_timeout_secs / 60,
            model.idle_timeout_secs % 60
        ),
        "(←/→ ±30s, [/] ±5m)",
        f == TuiField::IdleTimeout,
    );
    render_slider_row(
        b,
        "[5] Scene Dwell:",
        &format!("{:.1} s per scene", model.dwell_secs),
        "(←/→ ±0.5s, [/] ±2s)",
        f == TuiField::DwellDuration,
    );
    render_slider_row(
        b,
        "[6] Frame Rate:",
        &format_fps_display(model.fps),
        "(Auto / 30 / 60 / 120 / 144)",
        f == TuiField::FrameRate,
    );
    b.push_str("\x1b[K\r\n");

    let btn = |focused: bool, color: &str, label: &str| {
        if focused {
            format!("\x1b[7;1;{color}m [ {label} ] \x1b[0m")
        } else {
            format!("\x1b[1;{color}m [ {label} ] \x1b[0m")
        }
    };
    let s_b = btn(f == TuiField::SaveButton, "32", "S: Save");
    let p_b = btn(f == TuiField::PreviewButton, "36", "P: Terminal Preview");
    let f_b = btn(
        f == TuiField::FullscreenButton,
        "35",
        "F: Fullscreen Preview",
    );
    let q_b = btn(f == TuiField::QuitButton, "31", "Q: Quit");
    let _ = writeln!(b, "  {s_b}  {p_b}  {f_b}  {q_b}\x1b[K\r\n\x1b[K\r");

    let dirty_tag = if model.dirty {
        "\x1b[1;33m[Modified]\x1b[0m "
    } else {
        ""
    };
    let _ = writeln!(b, "  Status: {dirty_tag}{}\x1b[K\r", model.status);
    b.push_str("  \x1b[2mKeys: [↑↓/jk] Nav  [←→/hl] Adjust  [S] Save  [P] Term  [F] Fullscreen  [T] Standby  [Q] Quit\x1b[0m\x1b[K\r\n\x1b[J\x1b[?2026l");

    out.write_all(b.as_bytes())?;
    out.flush()
}

fn render_slider_row(b: &mut String, label: &str, val: &str, hint: &str, focused: bool) {
    let lbl_style = if focused { "\x1b[1;33m" } else { "\x1b[1m" };
    let bar = if focused {
        format!("\x1b[7m [ ◄  {val:^20}  ► ] \x1b[0m")
    } else {
        format!(" [ ◄  {val:^20}  ► ] ")
    };
    let _ = writeln!(
        b,
        "{lbl_style}{label:<18}\x1b[0m {bar} \x1b[2m{hint}\x1b[0m\x1b[K\r"
    );
}

pub fn fit_to_width(text: &str, cursor_idx: usize, max_cols: usize, editing: bool) -> String {
    let total_w = UnicodeWidthStr::width(text);
    if total_w <= max_cols {
        let mut res = text.to_string();
        res.extend(std::iter::repeat_n(' ', max_cols - total_w));
        return res;
    }
    let chars: Vec<char> = text.chars().collect();
    let start_idx = if editing {
        let mut w = 0usize;
        let mut s = cursor_idx.min(chars.len());
        while s > 0 {
            let cw = UnicodeWidthChar::width(chars[s - 1]).unwrap_or(0);
            if w + cw > max_cols.saturating_sub(2) {
                break;
            }
            w += cw;
            s -= 1;
        }
        s
    } else {
        0
    };
    let mut out = String::new();
    let mut used = 0usize;
    for &ch in &chars[start_idx..] {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > max_cols {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.extend(std::iter::repeat_n(' ', max_cols.saturating_sub(used)));
    out
}
