// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Control-socket request handling.
//!
//! The control socket is a local, unprivileged command channel: it must
//! never block the event loop, and it must never wait forever for a client.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use crate::daemon::dbus::ScreenSaverState;

/// How long a control client may take to send its command.
const READ_TIMEOUT: Duration = Duration::from_millis(500);

/// Reads one command from `stream` and dispatches it.
///
/// Returns true when the command asked the daemon to stop.
pub fn handle_control_client(stream: &mut UnixStream, state: &Arc<ScreenSaverState>) -> bool {
    // Bound the read: a client that connects and never sends would
    // otherwise block the event thread — and with it the daemon's signal
    // handling — indefinitely.
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let mut reader = BufReader::new(stream.take(4096));
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return false;
    }
    let cmd = line.trim();
    if cmd.is_empty() {
        return false;
    }
    let (resp, should_stop) = state.dispatch_control_command(cmd);
    let inner = reader.into_inner().into_inner();
    let _ = writeln!(inner, "{resp}");
    should_stop
}
