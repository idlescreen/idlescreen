//! Host system information. Vendored and slimmed from `runner::toolkit::sys_info`.
//!
//! Public API: `get_system_info`, `query_dark_mode`, `query_local_ip`,
//! `query_disk_drives` (delegated to `linux_queries`), `query_current_palette`.

#![allow(dead_code)]

use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub use crate::toolkit::platform::{
    DiskDriveInfo, NetworkAdapterInfo, PowerStatus, SystemBiosInfo, SystemInfo,
};

#[path = "linux_proc.rs"]
#[cfg(target_os = "linux")]
mod linux_proc;
#[path = "linux_queries.rs"]
mod linux_queries;

use crate::toolkit::theme_query::load_global_theme;

pub use linux_queries::{
    query_all_monitors as linux_query_all_monitors, query_disk_drives, query_gpu_names,
};

static DARK_MODE_CACHE: OnceLock<Mutex<(Option<bool>, Instant)>> = OnceLock::new();
static SYSTEM_INFO_CACHE: OnceLock<Mutex<(Option<SystemInfo>, Instant)>> = OnceLock::new();
static SYSTEM_OBJECT: OnceLock<Mutex<sysinfo::System>> = OnceLock::new();

fn get_system() -> std::sync::MutexGuard<'static, sysinfo::System> {
    SYSTEM_OBJECT
        .get_or_init(|| Mutex::new(sysinfo::System::new_all()))
        .lock()
        .unwrap()
}

/// Returns rich live system info. Cross-platform. Cached for 3 seconds.
pub fn get_system_info() -> SystemInfo {
    let cache_mutex = SYSTEM_INFO_CACHE.get_or_init(|| Mutex::new((None, Instant::now())));
    let mut cache = cache_mutex.lock().unwrap();
    if let Some(ref val) = cache.0 {
        if cache.1.elapsed() < Duration::from_secs(3) {
            return val.clone();
        }
    }
    let val = get_system_info_raw();
    cache.0 = Some(val.clone());
    cache.1 = Instant::now();
    val
}

fn get_system_info_raw() -> SystemInfo {
    let mut sys = get_system();
    sys.refresh_all();

    let os = sysinfo::System::long_os_version().unwrap_or_else(|| "Linux".to_string());
    let logo_text = os.clone();
    let kernel = sysinfo::System::kernel_version().unwrap_or_else(|| "unknown".to_string());
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "localhost".to_string());
    let cpu = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "CPU".to_string());

    let total = sys.total_memory();
    let available = sys.available_memory();
    let used = total.saturating_sub(available);
    let mem_total_mb = total / (1024 * 1024);
    let mem_used_mb = used / (1024 * 1024);
    let mem_used_pct = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    let cpu_usage_pct = sys.global_cpu_info().cpu_usage();
    let uptime_secs = sysinfo::System::uptime();

    let power = query_power_status().unwrap_or_default();
    let power_status = if power.ac_online {
        "AC".to_string()
    } else {
        format!("{}% (Battery)", power.battery_percent)
    };
    let disks = query_disk_drives();
    let disk_summary = if let Some(d) = disks.first() {
        format!("{} ~{}G free", d.path, d.free_bytes / (1024 * 1024 * 1024))
    } else {
        "disks".to_string()
    };
    let gpus = query_gpu_names().join(", ");
    let gpus = if gpus.is_empty() {
        "GPU(s)".to_string()
    } else {
        gpus
    };
    let monitors = format!("{} monitor(s)", linux_query_all_monitors().len());

    SystemInfo {
        os,
        logo_text,
        kernel,
        hostname,
        cpu,
        uptime_secs,
        mem_used_mb,
        mem_total_mb,
        mem_used_pct,
        cpu_usage_pct,
        power_status,
        disk_summary,
        gpus,
        monitors,
    }
}

/// Detect dark mode preference. Cached for 3 seconds.
pub fn query_dark_mode() -> bool {
    let cache_mutex = DARK_MODE_CACHE.get_or_init(|| Mutex::new((None, Instant::now())));
    let mut cache = cache_mutex.lock().unwrap();
    if let Some(val) = cache.0 {
        if cache.1.elapsed() < Duration::from_secs(3) {
            return val;
        }
    }
    let val = query_dark_mode_raw();
    cache.0 = Some(val);
    cache.1 = Instant::now();
    val
}

fn query_dark_mode_raw() -> bool {
    if let (_, Some(dark)) = load_global_theme() {
        return dark;
    }
    linux_proc::query_dark_mode_linux()
}

/// Power status: AC online + battery percent.
pub fn query_power_status() -> Option<PowerStatus> {
    #[cfg(target_os = "linux")]
    {
        linux_proc::query_power_status_linux()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Find the host's primary outbound IP by opening a UDP socket to 8.8.8.8.
pub fn query_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

/// Query the system palette from accent + dark mode.
pub fn query_current_palette() -> crate::core::screen_palette::ScreenPalette {
    let (global_accent, global_dark) = load_global_theme();
    let dark = global_dark.unwrap_or_else(query_dark_mode);
    let accent = global_accent.unwrap_or((0, 245, 255));
    crate::core::screen_palette::ScreenPalette::from_system(accent, dark)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTheme {
    pub is_dark_mode: bool,
    pub is_high_contrast: bool,
    pub accent_color: (u8, u8, u8),
}

pub fn query_system_theme() -> SystemTheme {
    let (global_accent, global_dark) = load_global_theme();
    let dark = global_dark.unwrap_or_else(query_dark_mode);
    let accent = global_accent.unwrap_or((0, 245, 255));
    SystemTheme {
        is_dark_mode: dark,
        is_high_contrast: false,
        accent_color: accent,
    }
}

pub use trance_api::MonitorCellBounds;

static MONITOR_LAYOUT_CACHE: OnceLock<
    Mutex<Option<(Vec<MonitorCellBounds>, (usize, usize), Instant)>>,
> = OnceLock::new();

pub fn get_monitor_layouts(cols: usize, rows: usize) -> Vec<MonitorCellBounds> {
    let cache_mutex = MONITOR_LAYOUT_CACHE.get_or_init(|| Mutex::new(None));
    let mut cache = cache_mutex.lock().unwrap();
    if let Some((ref layouts, (cached_cols, cached_rows), last_query)) = *cache {
        if cached_cols == cols
            && cached_rows == rows
            && last_query.elapsed() < Duration::from_secs(5)
        {
            return layouts.clone();
        }
    }

    let mut computed_layouts = None;
    if let Some(xmonitors) = query_monitors_from_xrandr() {
        // Calculate total virtual bounding box
        let min_x = xmonitors
            .iter()
            .map(|&(_, _, _, x, _)| x)
            .min()
            .unwrap_or(0);
        let max_x = xmonitors
            .iter()
            .map(|&(_, w, _, x, _)| x + w as i32)
            .max()
            .unwrap_or(0);
        let min_y = xmonitors
            .iter()
            .map(|&(_, _, _, _, y)| y)
            .min()
            .unwrap_or(0);
        let max_y = xmonitors
            .iter()
            .map(|&(_, _, h, _, y)| y + h as i32)
            .max()
            .unwrap_or(0);

        let total_width = (max_x - min_x) as usize;
        let total_height = (max_y - min_y) as usize;

        if total_width > 0 && total_height > 0 {
            let mut layouts = Vec::new();
            for (is_primary, w, h, x, y) in xmonitors {
                let rel_x1 = x - min_x;
                let rel_x2 = x + w as i32 - min_x;
                let rel_y1 = y - min_y;
                let rel_y2 = y + h as i32 - min_y;

                let start_col = (rel_x1 as usize * cols) / total_width;
                let end_col = (rel_x2 as usize * cols) / total_width;
                let start_row = (rel_y1 as usize * rows) / total_height;
                let end_row = (rel_y2 as usize * rows) / total_height;

                layouts.push(MonitorCellBounds {
                    start_col: start_col.clamp(0, cols),
                    end_col: end_col.clamp(0, cols),
                    start_row: start_row.clamp(0, rows),
                    end_row: end_row.clamp(0, rows),
                    is_primary,
                });
            }
            computed_layouts = Some(layouts);
        }
    }

    let result = computed_layouts.unwrap_or_else(|| {
        // Fallback: single monitor spanning the full terminal
        vec![MonitorCellBounds {
            start_col: 0,
            end_col: cols,
            start_row: 0,
            end_row: rows,
            is_primary: true,
        }]
    });

    *cache = Some((result.clone(), (cols, rows), Instant::now()));
    result
}

pub fn get_primary_monitor_bounds(cols: usize, rows: usize) -> MonitorCellBounds {
    if trance_api::MONITOR_BOUNDS_CALLBACK.get().is_some() {
        return trance_api::get_primary_monitor_bounds(cols, rows);
    }

    let layouts = get_monitor_layouts(cols, rows);
    layouts
        .into_iter()
        .find(|l| l.is_primary)
        .unwrap_or(MonitorCellBounds {
            start_col: 0,
            end_col: cols,
            start_row: 0,
            end_row: rows,
            is_primary: true,
        })
}

pub fn is_secondary_monitor() -> bool {
    std::env::var("TRANCE_SECONDARY_MONITOR").is_ok()
}

pub fn query_monitors_from_xrandr() -> Option<Vec<(bool, u32, u32, i32, i32)>> {
    if let Ok(exe) = std::env::current_exe() {
        if exe.to_string_lossy().contains("/deps/") {
            return None;
        }
    }
    let output = std::process::Command::new("xrandr")
        .arg("--listmonitors")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut monitors = Vec::new();
    for line in stdout.lines() {
        if line.contains("Monitors:") || line.trim().is_empty() {
            continue;
        }
        let is_primary = line.contains('*');
        let mut geometry_token = None;
        for token in line.split_whitespace() {
            if token.contains('x') && token.contains('+') {
                geometry_token = Some(token);
                break;
            }
        }
        if let Some(token) = geometry_token {
            let parts: Vec<&str> = token.split('+').collect();
            if parts.len() >= 3 {
                let size_part = parts[0];
                let x_offset: i32 = parts[1].parse().unwrap_or(0);
                let y_offset: i32 = parts[2].parse().unwrap_or(0);

                let size_subparts: Vec<&str> = size_part.split('x').collect();
                if size_subparts.len() == 2 {
                    let w_part = size_subparts[0].split('/').next().unwrap_or("0");
                    let h_part = size_subparts[1].split('/').next().unwrap_or("0");
                    let width: u32 = w_part.parse().unwrap_or(0);
                    let height: u32 = h_part.parse().unwrap_or(0);

                    if width > 0 && height > 0 {
                        monitors.push((is_primary, width, height, x_offset, y_offset));
                    }
                }
            }
        }
    }
    if monitors.is_empty() {
        None
    } else {
        Some(monitors)
    }
}
