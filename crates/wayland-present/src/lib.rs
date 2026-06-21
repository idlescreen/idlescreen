// SPDX-License-Identifier: MIT

//! Fullscreen Wayland overlays using [`zwlr_layer_shell_v1`].
//!
//! [`OverlayPresenter`] draws layer-shell surfaces above application windows.
//! Phase 2 renders solid-color fills; later phases pipe screensaver frames through
//! the same presenter.
//!
//! [`zwlr_layer_shell_v1`]: https://wayland.app/protocols/wlr-layer-shell-unstable-v1

mod appearance;
mod output;
mod overlay;
mod presenter;

pub use appearance::OverlayAppearance;
pub use output::OutputLayout;
pub use presenter::OverlayPresenter;