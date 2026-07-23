//! Host system information. Vendored and slimmed from `runner::toolkit::sys_info`.
//!
//! Public API: `get_system_info`, `query_dark_mode`, `query_local_ip`,
//! `query_disk_drives` (delegated to `linux_queries`), `query_current_palette`.

#![allow(dead_code)]

mod monitors;
mod theme;

use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub use crate::toolkit::platform::{
    DiskDriveInfo, NetworkAdapterInfo, PowerStatus, SystemBiosInfo, SystemInfo,
};

#[path = "../linux_proc.rs"]
#[cfg(target_os = "linux")]
mod linux_proc;
#[path = "../linux_queries.rs"]
mod linux_queries;

pub use linux_queries::{
    query_all_monitors as linux_query_all_monitors, query_disk_drives, query_gpu_names,
};
pub use monitors::{
    get_monitor_layouts, get_primary_monitor_bounds, is_secondary_monitor,
    query_monitors_from_xrandr,
};
pub use theme::{SystemTheme, query_current_palette, query_dark_mode, query_system_theme};
pub use trance_api::MonitorCellBounds;

use std::sync::RwLock;

static SYSTEM_INFO_CACHE: OnceLock<RwLock<(Option<SystemInfo>, Instant)>> = OnceLock::new();
static SYSTEM_OBJECT: OnceLock<Mutex<sysinfo::System>> = OnceLock::new();

fn get_system() -> std::sync::MutexGuard<'static, sysinfo::System> {
    SYSTEM_OBJECT
        .get_or_init(|| Mutex::new(sysinfo::System::new_all()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Returns rich live system info. Cross-platform. Cached for 3 seconds.
pub fn get_system_info() -> SystemInfo {
    let cache_rw = SYSTEM_INFO_CACHE.get_or_init(|| RwLock::new((None, Instant::now())));
    if let Ok(read_guard) = cache_rw.read()
        && let Some(ref val) = read_guard.0
        && read_guard.1.elapsed() < Duration::from_secs(3)
    {
        return val.clone();
    }
    let mut cache = cache_rw.write().unwrap_or_else(|e| e.into_inner());
    if let Some(ref val) = cache.0
        && cache.1.elapsed() < Duration::from_secs(3)
    {
        return val.clone();
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
    let kernel = sysinfo::System::kernel_version().unwrap_or_else(|| "unknown".to_string());
    let kernel_short = kernel.split('-').next().unwrap_or(&kernel);
    let logo_text = format!("Linux {}", kernel_short);
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

    let cpu_usage_pct = sys.global_cpu_usage();
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
