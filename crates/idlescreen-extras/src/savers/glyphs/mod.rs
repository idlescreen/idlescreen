//! Consolidated glyphs screensaver effect module.
//!
//! **Taxonomy Classification**: System Role (Purpose - Application Software).

use crate::runner::LcgRng;
use crate::runner::{get_system_info, query_current_palette};

mod draw;
mod physics;
mod screensaver_impl;

pub(crate) const CHAR_POOL_KATAKANA: &str =
    "ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ1234567890X:+-*<>|";

#[derive(Debug, Clone)]
pub struct RainDrop {
    pub x: f32,
    pub y: f32,
    pub speed: f32,
    pub length: usize,
    pub char_rot: usize,
    /// 0 = far (dimmer/slower), 1 = near (brighter/faster)
    pub layer: u8,
    /// Per-column speed phase so density/motion breathes unevenly.
    pub speed_phase: f32,
}

pub struct Glyphs {
    pub(super) rng: LcgRng,
    pub(super) drops: Vec<RainDrop>,
    pub(super) char_pool: Vec<char>,
    pub(super) last_cols: usize,
    pub(super) last_rows: usize,
    pub(super) density_opt: u32,

    pub(crate) sys_refresh_timer: f32,
    pub(super) mem_pressure: f32,
    pub(super) live_system_chars: Vec<char>,
    pub(super) time_elapsed: f32,
    pub(super) on_battery: bool,
    pub(super) frame_time_ema: f32,
    pub(super) quality_scale: f32,
    pub(super) target_frame_time: f32,
    pub(super) logo_text: String,
    pub(super) cached_accent: (u8, u8, u8),
    /// 0→1 fade-in after init / resize (~0.45s)
    pub(super) intro_fade: f32,
}

impl Default for Glyphs {
    fn default() -> Self {
        Self::new()
    }
}

impl Glyphs {
    pub fn new() -> Self {
        let mut char_pool: Vec<char> = CHAR_POOL_KATAKANA.chars().collect();
        let mut rng = LcgRng::from_env_or_random();
        let sys = get_system_info();
        let live_system_chars: Vec<char> = sys
            .hostname
            .chars()
            .chain(sys.os.chars())
            .chain(sys.kernel.chars())
            .filter(char::is_ascii_alphanumeric)
            .collect();

        // Seed pool with live system identity so machine-specific glyphs surface often.
        if !live_system_chars.is_empty() {
            for _ in 0..96 {
                let idx = rng.next_usize(live_system_chars.len());
                char_pool.push(live_system_chars[idx]);
            }
        }

        for _ in 0..160 {
            let idx = rng.next_usize(char_pool.len().max(1));
            if idx < char_pool.len() {
                char_pool.push(char_pool[idx]);
            }
        }

        let on_battery = sys.power_status.contains("Battery");

        Self {
            rng,
            drops: Vec::new(),
            char_pool,
            last_cols: 0,
            last_rows: 0,
            density_opt: 1,
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            live_system_chars,
            time_elapsed: 0.0,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            logo_text: sys.logo_text,
            cached_accent: query_current_palette().accent,
            intro_fade: 0.0,
        }
    }

    /// Pin the saver into a stable state for bench harness measurements.
    /// See `cosmos::Cosmos::prepare_for_bench` for the rationale; the
    /// important bit is `sys_refresh_timer = -1000.0` to suppress the
    /// slow system-info probe inside `update()`.
    pub fn prepare_for_bench(&mut self, cols: usize, rows: usize) {
        self.sys_refresh_timer = -1000.0;
        self.last_cols = cols;
        self.last_rows = rows;
    }
}

#[cfg(test)]
#[path = "glyphs_tests.rs"]
mod tests;
