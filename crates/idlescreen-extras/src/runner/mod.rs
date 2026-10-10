// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Runner runtime environment and primitives for mathematical screensavers.

use std::time::Duration;

pub mod audio {
    pub const NUM_AUDIO_BANDS: usize = 4;
    pub fn query_audio_bands() -> [f32; NUM_AUDIO_BANDS] {
        [0.0; NUM_AUDIO_BANDS]
    }
}

pub mod perf_test_support {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static COUNTING: Cell<bool> = const { Cell::new(false) };
        static ALLOCS: Cell<u64> = const { Cell::new(0) };
    }

    pub struct CountingAlloc;

    #[global_allocator]
    static COUNTING_ALLOC: CountingAlloc = CountingAlloc;

    unsafe impl GlobalAlloc for CountingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            bump();
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            bump();
            unsafe { System.realloc(ptr, layout, new_size) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            bump();
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

    pub fn count_allocs<T, F: FnOnce() -> T>(f: F) -> (T, u64) {
        ALLOCS.with(|n| n.set(0));
        COUNTING.with(|c| c.set(true));
        let out = f();
        COUNTING.with(|c| c.set(false));
        (out, ALLOCS.with(std::cell::Cell::get))
    }

    #[track_caller]
    pub fn assert_no_alloc<T, F: FnOnce() -> T>(what: &str, f: F) -> T {
        let (out, n) = count_allocs(f);
        assert_eq!(n, 0, "{what} allocated {n} time(s); its label promises none");
        out
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
            .map_or(1, |d| d.as_nanos() as u64);
        Self::new(seed)
    }

    pub fn from_env_or_random() -> Self {
        for key in &["RENDER_SEED", "IDLE_RENDER_SEED", "IDLE_SEED"] {
            if let Ok(raw) = std::env::var(key) {
                let s = raw.trim();
                if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                    if let Ok(v) = u64::from_str_radix(hex, 16) {
                        return Self::new(v);
                    }
                } else if let Ok(v) = s.parse::<u64>() {
                    return Self::new(v);
                }
            }
        }
        Self::new_random()
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
            0
        } else {
            (self.next_u64() % max as u64) as usize
        }
    }

    pub fn next_bool(&mut self, prob: f32) -> bool {
        self.next_f32() < prob
    }
}

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

pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let l = f32::midpoint(max, min);
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

pub fn lerp(a: f32, b: f32, factor: f32) -> f32 {
    let clamped_factor = factor.clamp(0.0, 1.0);
    a + (b - a) * clamped_factor
}

pub fn percentage(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        ((used as f64 / total as f64) * 100.0) as f32
    }
}

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
        Self {
            bg: (0, 0, 0),
            fg: (248, 248, 242),
            accent: (46, 204, 113),
            dim: (16, 71, 39),
            hot: (255, 140, 0),
            cool: (0, 180, 255),
            mid: (128, 128, 128),
            peak: (255, 255, 255),
        }
    }
}

pub fn query_current_palette() -> ScreenPalette {
    ScreenPalette::default()
}

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
        let os = detect_host_os().unwrap_or_else(|| "Linux".to_string());
        let kernel = detect_kernel().unwrap_or_else(|| "Linux".to_string());
        Self {
            logo_text: os.clone(),
            os,
            kernel,
            hostname: "localhost".to_string(),
            cpu: "CPU".to_string(),
            uptime_secs: 3600,
            mem_used_mb: 2048,
            mem_total_mb: 8192,
            mem_used_pct: 25.0,
            cpu_usage_pct: 5.0,
            power_status: "AC".to_string(),
            disk_summary: "OK".to_string(),
            gpus: "GPU".to_string(),
            monitors: "1".to_string(),
        }
    }
}

pub fn get_system_info() -> SystemInfo {
    SystemInfo::default()
}

pub fn detect_host_os() -> Option<String> {
    if let Ok(v) = std::env::var("IDLE_OS_NAME") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    for path in ["/etc/os-release", "/usr/lib/os-release"] {
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines() {
                let trimmed = line.trim();
                if let Some(v) = trimmed.strip_prefix("PRETTY_NAME=") {
                    let s = v.trim().trim_matches('"').trim_matches('\'').trim();
                    if !s.is_empty() {
                        return Some(s.to_string());
                    }
                }
            }
            for line in text.lines() {
                let trimmed = line.trim();
                if let Some(v) = trimmed.strip_prefix("NAME=") {
                    let s = v.trim().trim_matches('"').trim_matches('\'').trim();
                    if !s.is_empty() {
                        return Some(s.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn detect_desktop_environment() -> Option<String> {
    for key in ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"] {
        if let Ok(v) = std::env::var(key) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

pub fn detect_kernel() -> Option<String> {
    if let Ok(v) = std::env::var("IDLE_KERNEL_VERSION") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Ok(text) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        let t = text.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    None
}

pub fn publish_caption(_text: &str) {}

pub fn env_is_set(keys: &[&str]) -> bool {
    keys.iter().any(|k| std::env::var_os(k).is_some())
}

pub fn param(name: &str) -> Option<String> {
    let key = name.replace(['.', '-'], "_").to_ascii_uppercase();
    std::env::var(format!("IDLE_{key}")).ok()
}

pub fn param_f32(name: &str) -> Option<f32> {
    param(name).and_then(|v| v.parse::<f32>().ok())
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

pub fn span_reach_scale(cols: usize, rows: usize) -> f32 {
    let _ = (cols, rows);
    1.0
}

#[derive(Debug, Clone)]
pub struct CenteredLogo {
    pub lines: Vec<String>,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

fn get_5x5_pattern(ch: char) -> Option<[&'static str; 5]> {
    let u = ch.to_ascii_uppercase();
    match u {
        'A' => Some([" █▀▀█ ", "█   █", "█████", "█   █", "█   █"]),
        'B' => Some(["████  ", "█   █ ", "████  ", "█   █ ", "████  "]),
        'C' => Some([" ████", "█    ", "█    ", "█    ", " ████"]),
        'D' => Some(["████  ", "█   █ ", "█   █ ", "█   █ ", "████  "]),
        'E' => Some(["█████", "█    ", "████ ", "█    ", "█████"]),
        'F' => Some(["█████", "█    ", "████ ", "█    ", "█    "]),
        'G' => Some([" ████", "█    ", "█ ███", "█   █", " ████"]),
        'H' => Some(["█   █", "█   █", "█████", "█   █", "█   █"]),
        'I' => Some(["█████", "  █  ", "  █  ", "  █  ", "█████"]),
        'J' => Some(["    █", "    █", "    █", "█   █", " ███ "]),
        'K' => Some(["█   █", "█  █ ", "███  ", "█  █ ", "█   █"]),
        'L' => Some(["█    ", "█    ", "█    ", "█    ", "█████"]),
        'M' => Some(["█   █", "██ ██", "█ █ █", "█   █", "█   █"]),
        'N' => Some(["█   █", "██  █", "█ █ █", "█  ██", "█   █"]),
        'O' => Some([" ███ ", "█   █", "█   █", "█   █", " ███ "]),
        'P' => Some(["████ ", "█   █", "████ ", "█    ", "█    "]),
        'Q' => Some([" ███ ", "█   █", "█ █ █", "█  █ ", " ████"]),
        'R' => Some(["████ ", "█   █", "████ ", "█  █ ", "█   █"]),
        'S' => Some([" ████", "█    ", " ███ ", "    █", "████ "]),
        'T' => Some(["█████", "  █  ", "  █  ", "  █  ", "  █  "]),
        'U' => Some(["█   █", "█   █", "█   █", "█   █", " ███ "]),
        'V' => Some(["█   █", "█   █", " █ █ ", " █ █ ", "  █  "]),
        'W' => Some(["█   █", "█   █", "█ █ █", "██ ██", "█   █"]),
        'X' => Some(["█   █", " █ █ ", "  █  ", " █ █ ", "█   █"]),
        'Y' => Some(["█   █", " █ █ ", "  █  ", "  █  ", "  █  "]),
        'Z' => Some(["█████", "   █ ", "  █  ", " █   ", "█████"]),
        '0' => Some([" ███ ", "█  ██", "█ █ █", "██  █", " ███ "]),
        '1' => Some(["  █  ", " ██  ", "  █  ", "  █  ", "█████"]),
        '2' => Some([" ███ ", "█   █", "   █ ", " █   ", "█████"]),
        '3' => Some(["████ ", "    █", " ███ ", "    █", "████ "]),
        '4' => Some(["█   █", "█   █", "█████", "    █", "    █"]),
        '5' => Some(["█████", "█    ", "████ ", "    █", "████"]),
        '6' => Some([" ███ ", "█    ", "████ ", "█   █", " ███ "]),
        '7' => Some(["█████", "    █", "   █ ", "  █  ", "  █  "]),
        '8' => Some([" ███ ", "█   █", " ███ ", "█   █", " ███ "]),
        '9' => Some([" ███ ", "█   █", " ████", "    █", " ███ "]),
        '_' => Some(["     ", "     ", "     ", "     ", "█████"]),
        '!' => Some(["  █  ", "  █  ", "  █  ", "     ", "  █  "]),
        ' ' => Some(["     ", "     ", "     ", "     ", "     "]),
        '.' => Some(["     ", "     ", "     ", "     ", "  █  "]),
        '-' => Some(["     ", "     ", " ███ ", "     ", "     "]),
        _ => Some([" ███ ", "█   █", "█   █", "█   █", " ███ "]),
    }
}

pub fn render_logo_block(text: &str, sub_text: Option<&str>) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut rows: Vec<String> = vec![String::new(); 5];
    for ch in &chars {
        let pattern = get_5x5_pattern(*ch).unwrap_or(["     "; 5]);
        for (i, line) in pattern.iter().enumerate() {
            rows[i].push_str(line);
            rows[i].push(' ');
        }
    }
    if let Some(sub) = sub_text {
        rows.push(String::new());
        rows.push(sub.to_string());
    }
    rows
}

pub fn place_centered_logo(
    cols: usize,
    rows: usize,
    text: &str,
    sub_text: Option<&str>,
) -> Option<CenteredLogo> {
    if is_secondary_monitor() {
        return None;
    }
    let primary = get_primary_monitor_bounds(cols, rows);
    let budget = primary.width() / 6;
    if budget == 0 {
        return None;
    }
    let trimmed: String = text.trim().chars().take(budget).collect();
    let trimmed_sub = sub_text.map(|t| t.trim().chars().take(budget).collect::<String>());
    let lines = render_logo_block(&trimmed, trimmed_sub.as_deref());
    let logo_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let logo_h = lines.len();
    if logo_w == 0 || logo_h == 0 {
        return None;
    }
    let x = primary.start_col + primary.width().saturating_sub(logo_w) / 2;
    let y = primary.start_row + primary.height().saturating_sub(logo_h) / 2;
    Some(CenteredLogo {
        lines,
        x,
        y,
        width: logo_w,
        height: logo_h,
    })
}
