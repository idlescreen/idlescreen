//! System information polling and theme accent updates for Beams.

use super::types::Spotlight;
use crate::runner::{get_system_info, query_current_palette};

pub struct SysMetrics {
    pub mem_pressure: f32,
    pub cpu_load: f32,
    pub on_battery: bool,
    pub logo_text: String,
    pub cached_accent: (u8, u8, u8),
}

pub fn poll_sys_info(spotlights: &mut [Spotlight], host_bias: f32) -> SysMetrics {
    let sys = get_system_info();
    let mem_pressure = sys.mem_used_pct / 100.0;
    let cpu_load = (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0);
    let on_battery = sys.power_status.contains("Battery");
    let logo_text = sys.logo_text.clone();
    let cached_accent = query_current_palette().accent;

    for (i, spot) in spotlights.iter_mut().enumerate() {
        if i == 1 {
            spot.color_r = cached_accent.0 as f32;
            spot.color_g = cached_accent.1 as f32;
            spot.color_b = cached_accent.2 as f32;
        }
        let biased_load = cpu_load + (host_bias - 0.5) * 0.15;
        let load_factor = 1.0 + biased_load * 0.7 + mem_pressure * 0.5;
        spot.speed = (spot.speed * 0.85 + (0.55 + load_factor * 0.45) * 0.15).clamp(0.28, 2.6);
        spot.spread = (0.12 + mem_pressure * 0.08 + biased_load * 0.03).clamp(0.09, 0.30);
    }

    SysMetrics {
        mem_pressure,
        cpu_load,
        on_battery,
        logo_text,
        cached_accent,
    }
}
