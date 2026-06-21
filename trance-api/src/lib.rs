use std::sync::OnceLock;
use std::time::Duration;

/// A single cell in a character-grid renderer.
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

/// Linear Congruential Generator. Deterministic, lock-free.
#[derive(Clone, Debug)]
pub struct LcgRng(u64);

impl LcgRng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    pub fn new_random() -> Self {
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self::new(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    pub fn next_f32(&mut self) -> f32 {
        let val = (self.next_u64() >> 40) as u32;
        (val as f32) * (1.0 / (1u32 << 24) as f32)
    }
    pub fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
    pub fn next_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u64() % max as u64) as usize
    }
    pub fn next_bool(&mut self, prob: f32) -> bool {
        self.next_f32() < prob
    }
}

/// HSL→RGB conversion.
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r_prime + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g_prime + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b_prime + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

/// RGB→HSL conversion.
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let l = (max + min) / 2.0;
    let mut h = 0.0;
    let mut s = 0.0;
    if d > 0.0001 {
        s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        if max == r {
            h = (g - b) / d + (if g < b { 6.0 } else { 0.0 });
        } else if max == g {
            h = (b - r) / d + 2.0;
        } else {
            h = (r - g) / d + 4.0;
        }
        h *= 60.0;
    }
    (h, s, l)
}

/// Calculate percentage from two unsigned integers. Returns 0.0 if total is 0.
pub fn percentage(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (used as f32 / total as f32) * 100.0
    }
}

/// Linear interpolation between two values. Factor clamped to [0, 1].
pub fn lerp(a: f32, b: f32, factor: f32) -> f32 {
    let clamped_factor = factor.clamp(0.0, 1.0);
    a + (b - a) * clamped_factor
}

/// Cross-platform "where are we running" descriptor.
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub logo_text: String,
    pub kernel: String,
    pub hostname: String,
    pub cpu: String,
    pub uptime_secs: u64,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub mem_used_pct: f32,
    pub cpu_usage_pct: f32,
    pub power_status: String,
    pub disk_summary: String,
    pub gpus: String,
    pub monitors: String,
}

impl Default for SystemInfo {
    fn default() -> Self {
        let mut os = "Linux".to_string();
        let mut logo_text = "local76".to_string();
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let val = line.split('=').nth(1).unwrap_or("").trim_matches('"');
                    if !val.is_empty() {
                        os = val.to_string();
                        logo_text = val.to_string();
                        break;
                    }
                }
            }
        }

        let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());

        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            os,
            logo_text,
            kernel,
            hostname,
            cpu: "CPU".to_string(),
            uptime_secs: 0,
            mem_used_mb: 1,
            mem_total_mb: 2,
            mem_used_pct: 50.0,
            cpu_usage_pct: 0.0,
            power_status: "AC".to_string(),
            disk_summary: "disks".to_string(),
            gpus: "GPU".to_string(),
            monitors: "1 monitor".to_string(),
        }
    }
}

/// The canonical apps 4.0 screen palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenPalette {
    pub bg: (u8, u8, u8),
    pub fg: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub dim: (u8, u8, u8),
    pub hot: (u8, u8, u8),
    pub cool: (u8, u8, u8),
    pub mid: (u8, u8, u8),
    pub peak: (u8, u8, u8),
}

impl Default for ScreenPalette {
    fn default() -> Self {
        Self::from_system((46, 204, 113), true)
    }
}

impl ScreenPalette {
    pub fn from_system(accent: (u8, u8, u8), is_dark_mode: bool) -> Self {
        if is_dark_mode {
            Self {
                bg: (0, 0, 0),
                fg: (248, 248, 242),
                accent,
                dim: dim_color(accent, 0.35),
                hot: hue_rotated(accent, 30.0, 0.55),
                cool: hue_rotated(accent, -120.0, 0.45),
                mid: (128, 128, 128),
                peak: (255, 255, 255),
            }
        } else {
            Self {
                bg: (252, 252, 250),
                fg: (40, 42, 54),
                accent,
                dim: dim_color(accent, 0.7),
                hot: hue_rotated(accent, 30.0, 0.55),
                cool: hue_rotated(accent, -120.0, 0.45),
                mid: (160, 160, 160),
                peak: (255, 255, 255),
            }
        }
    }

    pub fn high_contrast(accent: (u8, u8, u8), is_dark_mode: bool) -> Self {
        let mut p = Self::from_system(accent, is_dark_mode);
        if is_dark_mode {
            p.bg = (0, 0, 0);
            p.fg = (255, 255, 255);
        } else {
            p.bg = (255, 255, 255);
            p.fg = (0, 0, 0);
        }
        p
    }

    pub fn default_dark() -> Self {
        Self::from_system((0, 245, 255), true)
    }

    pub fn default_light() -> Self {
        Self::from_system((0, 180, 200), false)
    }
}

fn dim_color(color: (u8, u8, u8), factor: f32) -> (u8, u8, u8) {
    (
        (color.0 as f32 * factor) as u8,
        (color.1 as f32 * factor) as u8,
        (color.2 as f32 * factor) as u8,
    )
}

fn hue_rotated(color: (u8, u8, u8), delta_deg: f32, target_lightness: f32) -> (u8, u8, u8) {
    let (h, _s, _l) = rgb_to_hsl(color.0, color.1, color.2);
    let new_h = (h + delta_deg).rem_euclid(360.0);
    hsl_to_rgb(new_h, 0.95, target_lightness)
}

// ---------------------------------------------------------------------------
// Dynamic Callback Hooks (Inverted Dependency Injection)
// ---------------------------------------------------------------------------
pub static SYSTEM_INFO_CALLBACK: OnceLock<fn() -> SystemInfo> = OnceLock::new();
pub static PALETTE_CALLBACK: OnceLock<fn() -> ScreenPalette> = OnceLock::new();

/// Returns live system information by calling the host's registered callback.
pub fn get_system_info() -> SystemInfo {
    if let Some(callback) = SYSTEM_INFO_CALLBACK.get() {
        callback()
    } else {
        SystemInfo::default()
    }
}

/// Returns the current host's visual palette by calling the host's registered callback.
pub fn query_current_palette() -> ScreenPalette {
    if let Some(callback) = PALETTE_CALLBACK.get() {
        callback()
    } else {
        ScreenPalette::default()
    }
}

// ---------------------------------------------------------------------------
// Trait Definitions
// ---------------------------------------------------------------------------

pub trait Screensaver: ScreensaverState {
    fn init(&mut self, _cols: usize, _rows: usize) {}
    fn update(&mut self, dt: Duration, cols: usize, rows: usize);
    fn update_frame_time(&mut self, _dt: Duration) {}
    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize);
    fn has_scanlines(&self) -> bool {
        false
    }
}

/// FFI-safe wrapper around the Screensaver trait object.
pub struct ScreensaverInstance {
    pub inner: Box<dyn Screensaver>,
}

pub trait ScreensaverState {
    fn active(&self) -> bool;
    fn set_active(&mut self, active: bool);
    fn focused(&self) -> bool;
    fn set_focused(&mut self, focused: bool);
}

impl<T: Screensaver + ?Sized> ScreensaverState for T {
    fn active(&self) -> bool {
        true
    }
    fn set_active(&mut self, _active: bool) {}
    fn focused(&self) -> bool {
        true
    }
    fn set_focused(&mut self, _focused: bool) {}
}

pub mod caption;
pub mod layout;
pub mod logo_block;

pub use caption::{caption_text, clear_caption, publish_caption};
pub use layout::{is_span_layout, place_centered_logo, span_reach_scale, CenteredLogo};

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

pub static MONITOR_BOUNDS_CALLBACK: OnceLock<fn(usize, usize) -> MonitorCellBounds> =
    OnceLock::new();
pub static IS_SECONDARY_MONITOR_CALLBACK: OnceLock<fn() -> bool> = OnceLock::new();

pub fn get_primary_monitor_bounds(cols: usize, rows: usize) -> MonitorCellBounds {
    if let Some(bounds) = read_primary_bounds_from_env() {
        return bounds;
    }
    if let Some(callback) = MONITOR_BOUNDS_CALLBACK.get() {
        callback(cols, rows)
    } else {
        MonitorCellBounds {
            start_col: 0,
            end_col: cols,
            start_row: 0,
            end_row: rows,
            is_primary: true,
        }
    }
}

fn read_primary_bounds_from_env() -> Option<MonitorCellBounds> {
    let start_col = std::env::var("TRANCE_PRIMARY_START_COL").ok()?.parse().ok()?;
    let end_col = std::env::var("TRANCE_PRIMARY_END_COL").ok()?.parse().ok()?;
    let start_row = std::env::var("TRANCE_PRIMARY_START_ROW").ok()?.parse().ok()?;
    let end_row = std::env::var("TRANCE_PRIMARY_END_ROW").ok()?.parse().ok()?;
    Some(MonitorCellBounds {
        start_col,
        end_col,
        start_row,
        end_row,
        is_primary: true,
    })
}

pub fn publish_primary_bounds(bounds: MonitorCellBounds) {
    unsafe {
        std::env::set_var("TRANCE_PRIMARY_START_COL", bounds.start_col.to_string());
        std::env::set_var("TRANCE_PRIMARY_END_COL", bounds.end_col.to_string());
        std::env::set_var("TRANCE_PRIMARY_START_ROW", bounds.start_row.to_string());
        std::env::set_var("TRANCE_PRIMARY_END_ROW", bounds.end_row.to_string());
    }
}

pub fn clear_primary_bounds() {
    unsafe {
        std::env::remove_var("TRANCE_PRIMARY_START_COL");
        std::env::remove_var("TRANCE_PRIMARY_END_COL");
        std::env::remove_var("TRANCE_PRIMARY_START_ROW");
        std::env::remove_var("TRANCE_PRIMARY_END_ROW");
    }
}

pub fn is_secondary_monitor() -> bool {
    if let Some(callback) = IS_SECONDARY_MONITOR_CALLBACK.get() {
        callback()
    } else {
        std::env::var("TRANCE_SECONDARY_MONITOR").is_ok()
    }
}

// Compatibility module structures for minimal changes in screensaver ports
pub mod core {
    pub use crate::{
        hsl_to_rgb, lerp, percentage, rgb_to_hsl, LcgRng, Screensaver, ScreensaverState,
        TerminalCell,
    };
    pub mod screensaver {
        pub use crate::{Screensaver, ScreensaverState};
    }
    pub mod logo_block {
        pub use crate::logo_block::render_logo_block;
    }
}

pub mod toolkit {
    pub mod sys_info {
        pub use crate::{
            caption_text, clear_caption, get_primary_monitor_bounds, get_system_info,
            is_secondary_monitor, is_span_layout, place_centered_logo, publish_caption,
            query_current_palette, span_reach_scale, CenteredLogo, MonitorCellBounds, SystemInfo,
        };
    }
}
