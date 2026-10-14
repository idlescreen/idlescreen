// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Runner runtime environment and primitives for mathematical screensavers.

mod color_rng;
mod logo_block;
mod sys_info;

pub use color_rng::{
    LcgRng, ScreenPalette, hsl_to_rgb, lerp, percentage, query_current_palette, rgb_to_hsl,
};
pub use logo_block::{CenteredLogo, place_centered_logo, render_logo_block};
pub use sys_info::{
    SystemInfo, detect_desktop_environment, detect_host_os, detect_kernel, detect_power_status,
    env_is_set, get_system_info, param, param_f32, publish_caption,
};

use std::time::Duration;

pub mod audio {
    pub const NUM_AUDIO_BANDS: usize = 4;
    pub fn query_audio_bands() -> [f32; NUM_AUDIO_BANDS] {
        [0.0; NUM_AUDIO_BANDS]
    }
}

pub mod perf_test_support {
    #[cfg(test)]
    pub fn count_allocs<T, F: FnOnce() -> T>(f: F) -> (T, u64) {
        inner::ALLOCS.with(|n| n.set(0));
        inner::COUNTING.with(|c| c.set(true));
        let out = f();
        inner::COUNTING.with(|c| c.set(false));
        (out, inner::ALLOCS.with(std::cell::Cell::get))
    }

    #[cfg(not(test))]
    pub fn count_allocs<T, F: FnOnce() -> T>(f: F) -> (T, u64) {
        (f(), 0)
    }

    #[track_caller]
    pub fn assert_no_alloc<T, F: FnOnce() -> T>(what: &str, f: F) -> T {
        let (out, n) = count_allocs(f);
        assert_eq!(
            n, 0,
            "{what} allocated {n} time(s); its label promises none"
        );
        out
    }

    #[cfg(test)]
    #[allow(unsafe_code)]
    mod inner {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;

        thread_local! {
            pub static COUNTING: Cell<bool> = const { Cell::new(false) };
            pub static ALLOCS: Cell<u64> = const { Cell::new(0) };
        }

        pub struct CountingAlloc;

        #[global_allocator]
        static COUNTING_ALLOC: CountingAlloc = CountingAlloc;

        // SAFETY: Delegates directly to std::alloc::System with thread-local counter bump.
        unsafe impl GlobalAlloc for CountingAlloc {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                bump();
                // SAFETY: Forwards valid layout to System::alloc.
                unsafe { System.alloc(layout) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                // SAFETY: Forwards valid pointer and layout to System::dealloc.
                unsafe { System.dealloc(ptr, layout) }
            }
            unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
                bump();
                // SAFETY: Forwards valid pointer, layout, and new_size to System::realloc.
                unsafe { System.realloc(ptr, layout, new_size) }
            }
            unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
                bump();
                // SAFETY: Forwards valid layout to System::alloc_zeroed.
                unsafe { System.alloc_zeroed(layout) }
            }
        }

        #[inline]
        fn bump() {
            let _ = COUNTING.try_with(|c| {
                if c.get() {
                    let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
                }
            });
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TerminalCell {
    pub ch: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
    pub bold: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: (248, 248, 242),
            bg: (0, 0, 0),
            bold: false,
        }
    }
}

pub trait Screensaver: Send + 'static {
    fn init(&mut self, _cols: usize, _rows: usize) {}
    fn update(&mut self, dt: Duration, cols: usize, rows: usize);
    fn update_frame_time(&mut self, _dt: Duration) {}
    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize);
    fn has_scanlines(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorCellBounds {
    pub start_col: usize,
    pub end_col: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub is_primary: bool,
}

impl MonitorCellBounds {
    pub fn width(&self) -> usize {
        self.end_col.saturating_sub(self.start_col)
    }

    pub fn height(&self) -> usize {
        self.end_row.saturating_sub(self.start_row)
    }

    pub fn center_col(&self) -> usize {
        self.start_col + self.width() / 2
    }

    pub fn center_row(&self) -> usize {
        self.start_row + self.height() / 2
    }

    pub fn contains(&self, col: usize, row: usize) -> bool {
        col >= self.start_col && col < self.end_col && row >= self.start_row && row < self.end_row
    }
}

pub fn get_primary_monitor_bounds(cols: usize, rows: usize) -> MonitorCellBounds {
    MonitorCellBounds {
        start_col: 0,
        end_col: cols,
        start_row: 0,
        end_row: rows,
        is_primary: true,
    }
}

thread_local! {
    static TLS_SECONDARY_MONITOR: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

pub fn is_secondary_monitor() -> bool {
    if let Some(b) = TLS_SECONDARY_MONITOR.with(std::cell::Cell::get) {
        return b;
    }
    env_is_set(&["IDLE_SECONDARY_MONITOR"])
}

pub fn set_thread_secondary_monitor(is_sec: Option<bool>) {
    TLS_SECONDARY_MONITOR.with(|c| c.set(is_sec));
}

pub fn span_reach_scale(_cols: usize, _rows: usize) -> f32 {
    1.0
}
