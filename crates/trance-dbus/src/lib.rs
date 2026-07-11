// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

//! D-Bus API for the trance screensaver daemon (`io.github.ubermetroid.trance`).
//!
//! The daemon exports configuration, preview, inhibit, and status signals on the
//! session bus. [`TranceClient`] wraps the typed methods for applets and CLI tools;
//! [`DaemonStatus`] is the canonical status snapshot shared with consumers.
//!
//! ## Constants
//!
//! - [`SERVICE_NAME`] — bus name (`io.github.ubermetroid.trance`)
//! - [`OBJECT_PATH`] — object path (`/io/github/crateria/trance`)
//! - [`INTERFACE_NAME`] — interface name (same as service)
//!
//! Clients should prefer [`TranceClient`] over raw D-Bus for typed errors and
//! status decoding via [`DaemonStatus::from_map`].

pub mod client;
pub mod status;

pub use client::{TranceClient, daemon_available};
pub use status::DaemonStatus;

pub const SERVICE_NAME: &str = "io.github.ubermetroid.trance";
pub const OBJECT_PATH: &str = "/io/github/crateria/trance";
pub const INTERFACE_NAME: &str = "io.github.ubermetroid.trance";

// Status signals use HashMap payloads for forward-compatible applet parsing.
