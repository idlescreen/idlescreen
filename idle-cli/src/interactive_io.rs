// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared stdin/stdout helpers for the interactive control panel.

use std::io::{self, Write};

use anyhow::{Context, Result};

/// Print `prompt`, flush stdout, and read a single trimmed line from stdin.
pub(crate) fn read_prompted_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush().context("flushing stdout")?;
    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .context("reading selection from stdin")?;
    Ok(buf)
}

/// Parse a 1-based index in `1..=len` from a trimmed user string.
pub(crate) fn parse_one_based_index(raw: &str, len: usize) -> Option<usize> {
    let idx = raw.trim().parse::<usize>().ok()?;
    if (1..=len).contains(&idx) {
        Some(idx)
    } else {
        None
    }
}
