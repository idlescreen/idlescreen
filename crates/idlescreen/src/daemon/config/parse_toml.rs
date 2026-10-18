// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Zero-dependency TOML and legacy YAML configuration parser.

use std::fs;
use std::path::Path;

use super::{Config, parse_targets_from_mode};

pub(super) fn parse_toml_config(s: &str) -> Result<Config, String> {
    let (mut cfg, mut sec, mut lines) = (Config::default(), "", s.lines());
    while let Some(line) = lines.next() {
        let trimmed = strip_comment_and_trim(line);
        if trimmed.is_empty() {
            continue;
        }
        if let Some(hdr) = trimmed.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            sec = hdr.trim().trim_matches(['"', '\'']);
            continue;
        }
        let (k, v_raw) = trimmed.split_once('=').ok_or("invalid TOML line")?;
        let key = k.trim().trim_matches(['"', '\'']);
        let mut val_buf = v_raw.trim().to_string();
        while val_buf.starts_with('[') && !has_unquoted_closing_bracket(&val_buf) {
            let next_l = lines.next().ok_or("unclosed array")?;
            val_buf.push(' ');
            val_buf.push_str(strip_comment_and_trim(next_l));
        }
        apply_toml_kv(&mut cfg, sec, key, val_buf.trim())?;
    }
    Ok(cfg)
}

fn apply_toml_kv(cfg: &mut Config, sec: &str, key: &str, val: &str) -> Result<(), String> {
    let num = || val.replace('_', "");
    match (sec, key) {
        ("general" | "", "idle_enabled") => {
            cfg.general.idle_enabled = val.parse().map_err(|_| "bad bool")?
        }
        ("general" | "", "idle_timeout_secs") => {
            cfg.general.idle_timeout_secs = num().parse().map_err(|_| "bad u64")?
        }
        ("general" | "", "dpms_timeout_secs") => {
            cfg.general.dpms_timeout_secs = num().parse().map_err(|_| "bad u64")?
        }
        ("general" | "", "primary_output") => cfg.general.primary_output = unquote_toml_str(val)?,
        ("saver", "active") => cfg.saver.active = Some(unquote_toml_str(val)?),
        ("saver.ascii" | "ascii", "mode") => cfg.saver.ascii.mode = unquote_toml_str(val)?,
        ("saver.ascii" | "ascii", "targets") => {
            cfg.saver.ascii.targets = parse_toml_str_array(val)?
        }
        ("saver.ascii" | "ascii", "text") => cfg.saver.ascii.text = unquote_toml_str(val)?,
        ("saver.ascii" | "ascii", "fps") => {
            cfg.saver.ascii.fps = num().parse().map_err(|_| "bad u32")?
        }
        ("saver.ascii" | "ascii", "dwell_secs") => {
            cfg.saver.ascii.dwell_secs = num().parse().map_err(|_| "bad f32")?
        }
        ("saver.ascii" | "ascii", "dissolve_duration_secs") => {
            cfg.saver.ascii.dissolve_duration_secs = num().parse().map_err(|_| "bad f32")?
        }
        _ => {}
    }
    Ok(())
}

fn has_unquoted_closing_bracket(s: &str) -> bool {
    let (mut in_q, mut q_ch, mut esc) = (false, '"', false);
    for c in s.chars() {
        if esc {
            esc = false;
        } else if in_q {
            esc = c == '\\' && q_ch == '"';
            in_q = c != q_ch;
        } else if matches!(c, '"' | '\'') {
            (in_q, q_ch) = (true, c);
        } else if c == ']' {
            return true;
        }
    }
    false
}

fn parse_toml_str_array(val: &str) -> Result<Vec<String>, String> {
    let inner = val
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .ok_or("bad array")?;
    let (mut items, mut start, mut in_q, mut q_ch, mut esc) = (Vec::new(), 0, false, '"', false);
    for (i, c) in inner.char_indices() {
        if esc {
            esc = false;
        } else if in_q {
            esc = c == '\\' && q_ch == '"';
            in_q = c != q_ch;
        } else if matches!(c, '"' | '\'') {
            (in_q, q_ch) = (true, c);
        } else if c == ',' {
            let tok = inner[start..i].trim();
            if !tok.is_empty() {
                items.push(unquote_toml_str(tok)?);
            }
            start = i + 1;
        }
    }
    let tail = inner[start..].trim();
    if !tail.is_empty() {
        items.push(unquote_toml_str(tail)?);
    }
    Ok(items)
}

pub(super) fn quote_toml_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {}
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn unquote_toml_str(s: &str) -> Result<String, String> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        return Ok(inner.to_string());
    }
    let inner = s
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .ok_or("unquoted string")?;
    let (mut out, mut chars) = (String::with_capacity(inner.len()), inner.chars());
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next().ok_or("trailing escape")? {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000C}'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                esc @ ('u' | 'U') => {
                    let n = if esc == 'u' { 4 } else { 8 };
                    let hex: String = chars.by_ref().take(n).collect();
                    let cp = u32::from_str_radix(&hex, 16)
                        .ok()
                        .filter(|_| hex.len() == n)
                        .and_then(char::from_u32)
                        .ok_or("bad unicode escape")?;
                    out.push(cp);
                }
                o => out.push(o),
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn strip_comment_and_trim(line: &str) -> &str {
    let (mut in_q, mut q_ch, mut esc) = (false, '"', false);
    for (i, c) in line.char_indices() {
        if esc {
            esc = false;
        } else if in_q {
            esc = c == '\\' && q_ch == '"';
            in_q = c != q_ch;
        } else if matches!(c, '"' | '\'') {
            (in_q, q_ch) = (true, c);
        } else if c == '#' {
            return line[..i].trim();
        }
    }
    line.trim()
}

pub fn migrate_yaml_to_toml(yaml_path: &Path, toml_path: &Path) -> std::io::Result<Config> {
    let raw = fs::read_to_string(yaml_path)?;
    let (mut cfg, mut sec, mut in_tgts, mut tgts) = (Config::default(), "", false, vec![]);
    for line in raw.lines() {
        let trimmed = strip_comment_and_trim(line);
        if trimmed.is_empty() {
            continue;
        }
        if in_tgts && let Some(item) = trimmed.strip_prefix('-') {
            let v = item.trim().trim_matches(['"', '\'']);
            if !v.is_empty() {
                tgts.push(v.to_string());
            }
            continue;
        }
        in_tgts = false;
        let Some((k, v_raw)) = trimmed.split_once(':') else {
            continue;
        };
        let (key, val) = (k.trim(), v_raw.trim().trim_matches(['"', '\'']));
        if val.is_empty() {
            match (line.len() - line.trim_start().len(), key) {
                (0, _) => sec = key,
                (_, "ascii") if matches!(sec, "saver" | "screensaver" | "ascii") => sec = "ascii",
                (_, "targets") if sec == "ascii" => in_tgts = true,
                _ => {}
            }
            continue;
        }
        match (sec, key) {
            ("", "idle_timeout") | ("general", "idle_timeout_secs") => {
                cfg.general.idle_timeout_secs = val.parse().unwrap_or(120);
            }
            ("general", "idle_enabled") => cfg.general.idle_enabled = val == "true",
            ("general", "dpms_timeout_secs") => {
                cfg.general.dpms_timeout_secs = val.parse().unwrap_or(0);
            }
            ("general", "primary_output") => cfg.general.primary_output = val.to_string(),
            ("saver" | "screensaver", "active") => cfg.saver.active = Some(val.to_string()),
            ("ascii", "mode") => cfg.saver.ascii.mode = val.to_string(),
            ("ascii", "text") => cfg.saver.ascii.text = val.to_string(),
            ("ascii", "fps") => cfg.saver.ascii.fps = val.parse().unwrap_or(0),
            ("ascii", "dwell_secs") => cfg.saver.ascii.dwell_secs = val.parse().unwrap_or(5.0),
            ("ascii", "dissolve_duration_secs") => {
                cfg.saver.ascii.dissolve_duration_secs = val.parse().unwrap_or(0.7);
            }
            ("ascii", "targets") => {
                for s in val.trim_matches(['[', ']']).split(',') {
                    let clean = s.trim().trim_matches(['"', '\'']);
                    if !clean.is_empty() {
                        tgts.push(clean.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    cfg.saver.ascii.targets = if tgts.is_empty() {
        parse_targets_from_mode(&cfg.saver.ascii.mode)
    } else {
        tgts
    };
    let _ = cfg.save_to_path(toml_path);
    let _ = fs::rename(yaml_path, yaml_path.with_file_name("config.yaml.migrated"));
    Ok(cfg)
}
