// SPDX-License-Identifier: MIT

//! D-Bus API for the trance screensaver daemon (`com.local76.Trance`).

pub mod client;
pub mod status;

pub use client::{daemon_available, TranceClient};
pub use status::DaemonStatus;

pub const SERVICE_NAME: &str = "com.local76.Trance";
pub const OBJECT_PATH: &str = "/com/local76/Trance";
pub const INTERFACE_NAME: &str = "com.local76.Trance";