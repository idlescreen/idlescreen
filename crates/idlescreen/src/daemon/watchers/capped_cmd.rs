// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Timeout-bounded subprocess execution.
//!
//! The inhibitor probes shell out to `busctl`, `pactl` and `hyprctl`. A
//! wedged audio server or a hung compositor makes those calls block forever,
//! so every probe runs under a hard deadline: the child is killed and its
//! result discarded when it overruns.

use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Default deadline for a probe command.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(400);

/// Runs `cmd` to completion under `timeout`.
///
/// Returns `None` when the command could not be started, when it was
/// killed for overrunning the deadline, or when its output could not be
/// collected.
pub fn run_capped(cmd: &mut Command, timeout: Duration) -> Option<Output> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let mut stdout = child.stdout.take()?;

    // Drain stdout concurrently: a child that fills the pipe buffer would
    // otherwise block on write and never exit.
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break None,
        }
    };

    let out = reader.join().unwrap_or_default();
    status.map(|status| Output {
        status,
        stdout: out,
        stderr: Vec::new(),
    })
}
