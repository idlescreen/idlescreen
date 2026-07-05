// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 UberMetroid

//! D-Bus API for the trance screensaver daemon (`com.ubermetroid.Trance`).
//!
//! The daemon exports configuration, preview, inhibit, and status signals on the
//! session bus. [`TranceClient`] wraps the typed methods for applets and CLI tools;
//! [`DaemonStatus`] is the canonical status snapshot shared with consumers.
//!
//! ## Constants
//!
//! - [`SERVICE_NAME`] — bus name (`com.ubermetroid.Trance`)
//! - [`OBJECT_PATH`] — object path (`/com/ubermetroid/Trance`)
//! - [`INTERFACE_NAME`] — interface name (same as service)
//!
//! Clients should prefer [`TranceClient`] over raw D-Bus for typed errors and
//! status decoding via [`DaemonStatus::from_map`].

pub mod client;
pub mod status;

pub use client::{TranceClient, daemon_available};
pub use status::DaemonStatus;

pub const SERVICE_NAME: &str = "com.ubermetroid.Trance";
pub const OBJECT_PATH: &str = "/com/ubermetroid/Trance";
pub const INTERFACE_NAME: &str = "com.ubermetroid.Trance";

// Status signals use HashMap payloads for forward-compatible applet parsing.
