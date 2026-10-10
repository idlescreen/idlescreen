// SPDX-License-Identifier: MIT

// `Storm` constructor + small helpers (Default, new,
// prepare_for_bench). The `Screensaver` trait impl (init /
// update_frame_time / update / draw) lives in `screensaver_impl.rs`.

use crate::runner::{LcgRng, get_system_info, query_current_palette};

use super::{BirdState, Phase, Storm};

impl Default for Storm {
    fn default() -> Self {
        Self::new()
    }
}

impl Storm {
    pub fn new() -> Self {
        // Pre-4.1 HKEY_CURRENT_USER registry reads (DropCount, AssembleSpeed)
        // collapsed to defaults for the inline migration. Re-added in 4.2.
        let drop_count_opt: u32 = 1;
        let assemble_speed_opt: u32 = 1;

        let sys = get_system_info();
        let on_battery = sys.power_status.contains("Battery");

        Self {
            rng: LcgRng::from_env_or_random(),
            // Each of these is `clear()`-and-rebuilt per frame
            // (or per state transition). Pre-allocating to a
            // comfortable ceiling removes ~4–5 first-ramp realloc
            // cycles during saver warm-up. Caps were picked from
            // observing hot-loop profiles at 1080p:
            //   stars: 200 (clamp(15, 60) × cols/rows naturally
            //     grows; 200 covers 99% of cases)
            //   drops: 500 (drop_count_opt × quality × battery)
            //   splashes: 200 (rain + bird landings)
            //   logo_cells: 500 (small bitmap raster)
            //   bg_cells / mid_scenery / fg_scenery: 200 each
            //     (terminal-size placeholder set)
            //   lightning_bolts: 16 (per flash burst)
            //   perch_points: 16 (rare; trees + squirrels)
            stars: Vec::with_capacity(200),
            logo_cells: Vec::with_capacity(500),
            drops: Vec::with_capacity(500),
            splashes: Vec::with_capacity(200),
            phase: Phase::Building,
            phase_timer: 0.0,
            last_cols: 0,
            last_rows: 0,
            drop_count_opt,
            assemble_speed_opt,
            sys_refresh_timer: 0.0,
            mem_pressure: sys.mem_used_pct / 100.0,
            cpu_load: (sys.cpu_usage_pct / 100.0).clamp(0.0, 1.0),
            _host_bias: sys.hostname.chars().map(|c| c as u32).sum::<u32>() as f32 / 1000.0 % 1.0,
            on_battery,
            frame_time_ema: 0.01666667,
            quality_scale: 1.0,
            target_frame_time: 0.01666667,
            // `puddle` is `vec![0.0; cols]` per cols refresh — its
            // size is genuinely variable (terminal column count),
            // so leave it as Vec::new() and let it grow.
            puddle: Vec::new(),
            puddle_color: Vec::new(),
            wind: 0.0,
            lightning_timer: 0.0,
            lightning_flash: 0.0,
            lightning_bolts: Vec::with_capacity(16),
            lightning_is_background: false,
            lightning_delay: 0.0,
            bg_cells: Vec::with_capacity(200),
            mid_scenery: Vec::with_capacity(200),
            fg_scenery: Vec::with_capacity(200),
            bird_x: 0.0,
            bird_y: 0.0,
            bird_state: BirdState::Sitting,
            bird_timer: 0.0,
            bird_wing_flap: false,
            bird_vx: 0.0,
            bird_vy: 0.0,
            bird_perch_x: 0.0,
            bird_perch_y: 0.0,
            perch_points: Vec::with_capacity(16),
            active_animal: None,
            animal_spawn_timer: 28.0,
            subtitle: String::new(),
            subtitle_timer: 0.0,
            time_elapsed: 0.0,
            cached_accent: query_current_palette().accent,
            intro_fade: 0.0,
            wind_target: 0.0,
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
mod tests {
    use super::*;

    #[test]
    fn new_starts_in_building_phase() {
        let s = Storm::new();
        assert_eq!(s.phase, Phase::Building);
        assert_eq!(s.phase_timer, 0.0);
        assert_eq!(s.intro_fade, 0.0);
    }

    #[test]
    fn new_caps_match_documented_allocation_hints() {
        // The pre-allocation caps (from `new()`) must match the
        // document above the with_capacity calls so a future tweak
        // gets caught by code review or this test.
        let s = Storm::new();
        assert!(s.stars.capacity() >= 200);
        assert!(s.drops.capacity() >= 500);
        assert!(s.splashes.capacity() >= 200);
        assert!(s.logo_cells.capacity() >= 500);
        assert!(s.bg_cells.capacity() >= 200);
        assert!(s.mid_scenery.capacity() >= 200);
        assert!(s.fg_scenery.capacity() >= 200);
        assert!(s.lightning_bolts.capacity() >= 16);
        assert!(s.perch_points.capacity() >= 16);
    }

    #[test]
    fn default_matches_new() {
        // `Default::default()` must produce the same struct as
        // `Storm::new()` — both go through the same constructor.
        let a = Storm::default();
        let b = Storm::new();
        assert_eq!(a.phase, b.phase);
        assert_eq!(a.target_frame_time, b.target_frame_time);
        assert_eq!(a.drop_count_opt, b.drop_count_opt);
    }

    #[test]
    fn prepare_for_bench_sets_sys_refresh_timer_negative() {
        let mut s = Storm::new();
        s.prepare_for_bench(80, 24);
        assert_eq!(s.sys_refresh_timer, -1000.0);
        assert_eq!(s.last_cols, 80);
        assert_eq!(s.last_rows, 24);
    }
}
