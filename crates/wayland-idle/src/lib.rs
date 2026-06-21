// SPDX-License-Identifier: MIT

//! Wayland idle detection using the [`ext-idle-notify-v1`] protocol.
//!
//! Compositors such as COSMIC, Sway, Hyprland, and KWin expose this extension.
//! When a Wayland session is available, [`IdleMonitor`] connects to the compositor
//! and reports whether the user has been inactive longer than the configured timeout.
//!
//! [`ext-idle-notify-v1`]: https://wayland.app/protocols/ext-idle-notify-v1

mod monitor;
mod wayland;

pub use monitor::IdleMonitor;