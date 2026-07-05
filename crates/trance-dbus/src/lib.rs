// SPDX-License-Identifier: MIT

//! D-Bus API for the trance screensaver daemon (`com.local76.Trance`).
//!
//! **Org-rename note (2026):** The project was historically published
//! under the `local76` GitHub organization and the D-Bus bus name was
//! baked into the wire protocol. The current public repository is
//! `UberMetroid/trance`. **Renaming the bus name would silently break
//! every existing install** (the daemon would refuse to bind the same
//! name as the running process; clients using the old name would fail
//! to connect; `trance config set` and `trance status` would stop
//! working until the user runs a migration). Until a coordinated
//! migration is designed — see review
//! `/tmp/reviews/03-desktop-trance.md` item "Org-rename story is
//! incomplete" — these constants MUST stay as `com.local76.*` and the
//! install paths under `/usr/libexec/local76/screensavers/` MUST stay
//! intact.
//!
//! The daemon exports configuration, preview, inhibit, and status signals on the
//! session bus. [`TranceClient`] wraps the typed methods for applets and CLI tools;
//! [`DaemonStatus`] is the canonical status snapshot shared with consumers.
//!
//! ## Constants
//!
//! - [`SERVICE_NAME`] — bus name (`com.local76.Trance`)
//! - [`OBJECT_PATH`] — object path (`/com/local76/Trance`)
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

pub const LEGACY_SERVICE_NAME: &str = "com.local76.Trance";
pub const LEGACY_OBJECT_PATH: &str = "/com/local76/Trance";
pub const LEGACY_INTERFACE_NAME: &str = "com.local76.Trance";

// Status signals use HashMap payloads for forward-compatible applet parsing.
