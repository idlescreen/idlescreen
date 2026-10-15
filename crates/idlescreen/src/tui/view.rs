// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Flicker-free terminal rendering layout with synchronized output and Unicode display width.

use std::fmt::Write as FmtWrite;
use std::io::{Write, stdout};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::mini_preview::{MiniPreview, WIDE_TERM_MIN_COLS};
use super::model::{
    ModeChoice, TuiField, TuiModel, char_to_byte_idx, format_dpms_display, format_fps_display,
    format_saver_display,
};

pub fn render_view(model: &TuiModel) -> std::io::Result<()> {
    let mut buf = String::with_capacity(8192);
    render_view_into(model, &mut buf)
}

pub fn render_view_into(model: &TuiModel, b: &mut String) -> std::io::Result<()> {
    let term_cols = crossterm::terminal::size().map_or(80, |(c, _)| c as usize);
    render_view_with_preview_into(model, None, term_cols, b)
}

pub fn render_view_with_preview_into(
    model: &TuiModel,
    preview: Option<&MiniPreview>,
    term_cols: usize,
    b: &mut String,
) -> std::io::Result<()> {
    let mut out = stdout();
    format_view_buffer(model, preview, term_cols, b);
    out.write_all(b.as_bytes())?;
    out.flush()
}

pub fn format_view_buffer(
    model: &TuiModel,
    preview: Option<&MiniPreview>,
    term_cols: usize,
    b: &mut String,
) {
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
    let raw_in = &model.inhibitor_summary;
    let inhib = if term_cols >= WIDE_TERM_MIN_COLS && raw_in.len() > 14 {
        fit_to_width(raw_in, 0, 14, false)
    } else {
        raw_in.clone()
    };
    let _ = writeln!(
        b,
        "  Daemon: {d_col}{}\x1b[0m │ Inhibitors: \x1b[1m{}\x1b[0m \x1b[2m([T] Standby)\x1b[0m\x1b[K\r\n\x1b[K\r",
        model.daemon_status,
        inhib.trim_end()
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
    let s_line = if f == TuiField::ActiveSaver {
        format!("   \x1b[7;1;36m [ ◄  {saver_disp:^44}  ► ] \x1b[0m  \x1b[2m(←/→ or [/])\x1b[0m")
    } else {
        format!("    [ ◄  \x1b[1;36m{saver_disp:^44}\x1b[0m  ► ]   \x1b[2m(←/→ or [/])\x1b[0m")
    };
    let _ = writeln!(b, "{s_line}\x1b[K\r\n\x1b[K\r");

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
        let s_str = fit_to_width(sample, 0, 26, false);
        let lbl = m.label();
        let line = if is_focused {
            format!("{ptr} \x1b[7m {radio} {lbl:<24} [{s_str}] \x1b[0m")
        } else if is_selected {
            format!("{ptr} \x1b[1;32m{radio} {lbl:<24} [{s_str}]\x1b[0m")
        } else {
            format!("{ptr} \x1b[2m{radio}\x1b[0m {lbl:<24} \x1b[2m[{s_str}]\x1b[0m")
        };
        let _ = writeln!(b, "{line}\x1b[K\r");
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
    let st = if is_tf && model.editing_text {
        "\x1b[1m"
    } else if is_tf {
        "\x1b[7m"
    } else {
        ""
    };
    let _ = writeln!(b, "  {bdr}│\x1b[0m {st}{vis}\x1b[0m {bdr}│\x1b[0m\x1b[K\r");
    let _ = writeln!(
        b,
        "  {bdr}└──────────────────────────────────────────────────────────────┘\x1b[0m\x1b[K\r\n\x1b[K\r"
    );

    let idle_s = model.idle_timeout_secs;
    let sliders = [
        (
            "[4] Idle Timeout:",
            format!("{idle_s} s ({}m {:02}s)", idle_s / 60, idle_s % 60),
            "(←/→ ±30s, [/] ±5m)",
            TuiField::IdleTimeout,
        ),
        (
            "[5] Scene Dwell:",
            format!("{:.1} s per scene", model.dwell_secs),
            "(←/→ ±0.5s, [/] ±2s)",
            TuiField::DwellDuration,
        ),
        (
            "[6] Frame Rate:",
            format_fps_display(model.fps),
            "(←/→ Auto..144Hz)",
            TuiField::FrameRate,
        ),
        (
            "[7] DPMS Power-Off:",
            format_dpms_display(model.dpms_timeout_secs),
            "(←/→ ±1m, [/] ±5m)",
            TuiField::DpmsTimeout,
        ),
    ];
    for (lbl, val, hint, fld) in &sliders {
        render_slider_row(b, lbl, val, hint, f == *fld);
    }
    b.push_str("\x1b[K\r\n");

    let btn = |foc: bool, c: &str, l: &str| {
        let rev = if foc { "7;" } else { "" };
        format!("\x1b[{rev}1;{c}m [ {l} ] \x1b[0m")
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
    b.push_str("  \x1b[2mKeys: [↑↓/jk] Nav  [←→/hl] Adjust  [S] Save  [P] Term  [F] Fullscreen  [T] Standby  [Q] Quit\x1b[0m\x1b[K\r\n\x1b[J");

    if term_cols >= WIDE_TERM_MIN_COLS {
        if let Some(prev) = preview {
            prev.render_into(model, term_cols, b);
        } else {
            MiniPreview::new(model, term_cols).render_into(model, term_cols, b);
        }
    }
    b.push_str("\x1b[?2026l");
}

fn render_slider_row(b: &mut String, label: &str, val: &str, hint: &str, foc: bool) {
    let ls = if foc { "\x1b[1;33m" } else { "\x1b[1m" };
    let rev = if foc { "\x1b[7m" } else { "" };
    let end = if foc { "\x1b[0m" } else { "" };
    let _ = writeln!(
        b,
        "{ls}{label:<19}\x1b[0m {rev} [ ◄  {val:^20}  ► ] {end} \x1b[2m{hint}\x1b[0m\x1b[K\r"
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
        let (mut w, mut s) = (0usize, cursor_idx.min(chars.len()));
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
    let (mut out, mut used) = (String::new(), 0usize);
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
