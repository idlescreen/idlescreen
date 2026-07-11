// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

//! Lightweight shared API definitions, traits, and math utilities for terminal
//! screensaver plugins. Host applications register callbacks for live system
//! queries; plugins depend only on this crate for portable drawing primitives.
//!
//! ## Example
//!
//! ```rust
//! use trance_api::{LcgRng, Screensaver, TerminalCell};
//!
//! struct SolidRed;
//!
//! impl Screensaver for SolidRed {
//!     fn update(&mut self, _: std::time::Duration, _: usize, _: usize) {}
//!     fn draw(&self, grid: &mut [TerminalCell], _cols: usize, _rows: usize) {
//!         for cell in grid.iter_mut() {
//!             cell.bg = (255, 0, 0); // pure red
//!             cell.fg = (255, 0, 0);
//!         }
//!     }
//! }
//!
//! let mut rng = LcgRng::new(0xDEAD_BEEF);
//! let _ = rng.next_u64();
//! ```

mod callbacks;
mod caption;
mod color;
mod layout;
mod logo_block;
mod monitor;
mod palette;
mod rng;
mod screensaver;
mod system_info;
mod terminal_cell;

pub use callbacks::{
    PALETTE_CALLBACK, SYSTEM_INFO_CALLBACK, get_system_info, query_current_palette,
};
pub use caption::{caption_text, clear_caption, publish_caption};
pub use color::{hsl_to_rgb, lerp, percentage, rgb_to_hsl};
pub use layout::{CenteredLogo, is_span_layout, place_centered_logo, span_reach_scale};
pub use logo_block::render_logo_block;
pub use monitor::{
    IS_SECONDARY_MONITOR_CALLBACK, MONITOR_BOUNDS_CALLBACK, MonitorCellBounds,
    clear_primary_bounds, get_primary_monitor_bounds, is_secondary_monitor, publish_primary_bounds,
};
pub use palette::ScreenPalette;
pub use rng::LcgRng;
pub use screensaver::{GpuSpotlight, Screensaver, ScreensaverInstance, ScreensaverState};
pub use system_info::SystemInfo;
pub use terminal_cell::TerminalCell;

/// Compatibility module structures for minimal changes in screensaver ports.
pub mod core {
    pub use crate::{
        GpuSpotlight, LcgRng, Screensaver, ScreensaverState, TerminalCell, hsl_to_rgb, lerp,
        percentage, rgb_to_hsl,
    };
    pub mod screensaver {
        pub use crate::{GpuSpotlight, Screensaver, ScreensaverState};
    }
    pub mod logo_block {
        pub use crate::logo_block::render_logo_block;
    }
}

pub mod toolkit {
    pub mod sys_info {
        pub use crate::{
            CenteredLogo, MonitorCellBounds, SystemInfo, caption_text, clear_caption,
            get_primary_monitor_bounds, get_system_info, is_secondary_monitor, is_span_layout,
            place_centered_logo, publish_caption, query_current_palette, span_reach_scale,
        };
    }
}
