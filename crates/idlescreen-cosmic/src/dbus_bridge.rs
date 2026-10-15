// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Self-contained error, logging, discovery, and daemon bridge for the COSMIC applet.

use std::fmt;
use std::process::Command;

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<E: std::error::Error> From<E> for Error {
    fn from(err: E) -> Self {
        Self(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Context<T> {
    fn context(self, msg: &str) -> Result<T>;
}

impl<T, E: fmt::Display> Context<T> for std::result::Result<T, E> {
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| Error(format!("{msg}: {e}")))
    }
}

pub fn err_msg(msg: impl Into<String>) -> Error {
    Error(msg.into())
}

#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub idle_enabled: bool,
    pub idle_timeout_mins: u32,
    pub active_saver: String,
    pub show_fps_overlay: bool,
    pub render_scale: String,
}

impl Default for DaemonStatus {
    fn default() -> Self {
        Self {
            idle_enabled: true,
            idle_timeout_mins: 2,
            active_saver: "ascii".to_string(),
            show_fps_overlay: false,
            render_scale: "null".to_string(),
        }
    }
}

#[rustfmt::skip]
const BUILTIN_SCENES: &[&str] = &[
    "ascii", "superfedora", "tour",
    "aurora", "beams", "binarypath", "blackhole", "bouncyballs", "breakout",
    "bubbles", "burn", "chaos", "colorshift", "cosmos", "crumble",
    "decrypt", "errorcorrect", "expand", "fireworks", "highlight", "laseretch",
    "life", "matrix", "metaballs", "middleout", "mystify", "orbittingvolley",
    "overflow", "pipes", "plasma", "pour", "print", "radar",
    "rain", "randomsequence", "rings", "ripple", "scattered", "slice",
    "slide", "smoke", "spotlights", "spray", "swarm", "sweep",
    "synthgrid", "tetris", "thunderstorm", "tunnel", "unstable", "vhstape",
    "waves", "wipe",
];

pub fn detect_screensavers() -> Vec<String> {
    BUILTIN_SCENES.iter().map(|s| (*s).to_string()).collect()
}

pub fn daemon_available() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "idlescreen.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn start_service() -> Result<()> {
    let status = Command::new("systemctl")
        .args(["--user", "enable", "--now", "idlescreen.service"])
        .status()
        .context("systemctl enable --now idlescreen.service")?;
    if !status.success() {
        return Err(err_msg("systemctl enable --now idlescreen.service failed"));
    }
    Ok(())
}

pub fn stop_service() -> Result<()> {
    let status = Command::new("systemctl")
        .args(["--user", "stop", "idlescreen.service"])
        .status()
        .context("systemctl stop idlescreen.service")?;
    if !status.success() {
        return Err(err_msg("systemctl stop idlescreen.service failed"));
    }
    Ok(())
}

pub fn run_idlescreen(args: &[&str]) -> Result<String> {
    let out = Command::new("idlescreen")
        .args(args)
        .output()
        .context("failed to execute idlescreen CLI")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(err_msg(format!(
            "idlescreen {:?} exited with {}: {}",
            args,
            out.status,
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
