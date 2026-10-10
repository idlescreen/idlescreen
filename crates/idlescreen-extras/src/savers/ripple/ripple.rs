// SPDX-License-Identifier: MIT

// `Ripple` constructor + small public/internal helpers (Default,
// new, prepare_for_bench, impact, pick_weather). The Screensaver
// trait impl (init / update_frame_time / update / draw) lives in
// `screensaver_impl.rs`.

use crate::runner::{LcgRng, get_system_info, query_current_palette};

use super::{Ripple, Weather};

impl Default for Ripple {
    fn default() -> Self {
        Self::new()
    }
}

impl Ripple {
    pub fn new() -> Self {
        let sys = get_system_info();
        Self {
            rng: LcgRng::from_env_or_random(),
            time: 0.0,
            intro_fade: 0.0,
            last_cols: 0,
            last_rows: 0,
            rings: Vec::new(),
            drops: Vec::new(),
            splashes: Vec::new(),
            delayed: Vec::new(),
            rain_timer: 0.1,
            wind: 0.0,
            weather: Weather::Drizzle,
            weather_timer: 12.0,
            weather_intensity: 0.45,
            on_battery: sys.power_status.contains("Battery"),
            quality_scale: 1.0,
            frame_time_ema: 0.016,
            target_frame_time: 0.016,
            sys_timer: 0.0,
            accent: query_current_palette().accent,
            logo_text: sys.logo_text,
            surface_phase: 0.0,
            accum: std::cell::RefCell::new(Vec::new()),
            accent_a: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Pin the saver into a stable state for bench harness measurements.
    /// See `cosmos::Cosmos::prepare_for_bench` for the rationale; the
    /// important bit is `sys_refresh_timer = -1000.0` to suppress the
    /// slow system-info probe inside `update()`.
    pub fn prepare_for_bench(&mut self, cols: usize, rows: usize) {
        self.sys_timer = -1000.0;
        self.last_cols = cols;
        self.last_rows = rows;
    }

    /// Inject a ripple at a grid coordinate. Public API for plugin callers
    /// (and the unit tests); `cargo check --all-targets` counts tests as uses
    /// so this is reachable, but `cargo check` alone flags it as a warning.
    #[allow(dead_code)]
    pub fn impact(&mut self, x: f32, y: f32, strength: f32, cols: usize, rows: usize) {
        super::types::impact(
            &mut self.rings,
            &mut self.splashes,
            &mut self.delayed,
            &mut self.rng,
            self.weather,
            self.on_battery,
            self.quality_scale,
            x,
            y,
            strength,
            cols,
            rows,
        );
    }

    /// Roll a new weather state and prime the timer + intensity.
    pub fn pick_weather(&mut self) {
        let r = self.rng.next_f32();
        self.weather = if r < 0.22 {
            Weather::Lull
        } else if r < 0.62 {
            Weather::Drizzle
        } else {
            Weather::Shower
        };
        self.weather_timer = match self.weather {
            Weather::Lull => 6.0 + self.rng.next_f32() * 10.0,
            Weather::Drizzle => 10.0 + self.rng.next_f32() * 16.0,
            Weather::Shower => 5.0 + self.rng.next_f32() * 9.0,
        };
        self.weather_intensity = match self.weather {
            Weather::Lull => 0.15,
            Weather::Drizzle => 0.45,
            Weather::Shower => 0.9,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_default_weather_drizzle() {
        let r = Ripple::new();
        assert_eq!(r.weather, Weather::Drizzle);
        assert!(r.weather_timer > 0.0);
        assert_eq!(r.weather_intensity, 0.45);
    }

    #[test]
    fn default_matches_new() {
        // `Default::default()` must produce the same struct as
        // `Ripple::new()` — both go through the same constructor.
        assert_eq!(Ripple::default().weather_timer, Ripple::new().weather_timer);
    }

    #[test]
    fn pick_weather_stays_in_three_states() {
        // The three-state ternary must always pick one of {Lull,
        // Drizzle, Shower} regardless of the rng state.
        for _ in 0..16 {
            let mut r = Ripple::new();
            r.pick_weather();
            assert!(matches!(
                r.weather,
                Weather::Lull | Weather::Drizzle | Weather::Shower
            ));
        }
    }

    #[test]
    fn prepare_for_bench_sets_sys_timer_negative() {
        // The bench harness needs the sys_info probe suppressed; the
        // contract is `sys_timer = -1000.0`.
        let mut r = Ripple::new();
        r.prepare_for_bench(80, 24);
        assert_eq!(r.sys_timer, -1000.0);
        assert_eq!(r.last_cols, 80);
        assert_eq!(r.last_rows, 24);
    }
}
