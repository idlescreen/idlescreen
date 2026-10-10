// SPDX-License-Identifier: MIT
// `Screensaver` trait impl for `Ripple` — init, update_frame_time,
// update, draw. Each method is a self-contained entry point; they
// live in one page because the trait requires them co-located and
// they share the `&mut self` / `&self` receiver shape.

use std::time::Duration;

use crate::runner::{Screensaver, TerminalCell, get_system_info, query_current_palette};

use super::Ripple;
use super::types;

impl Screensaver for Ripple {
    fn init(&mut self, cols: usize, rows: usize) {
        self.intro_fade = 0.0;
        self.time = 0.0;
        self.last_cols = cols;
        self.last_rows = rows;
        self.rings.clear();
        self.drops.clear();
        self.splashes.clear();
        self.delayed.clear();
        self.rain_timer = 0.05;
        self.wind = (self.rng.next_f32() - 0.5) * 0.6;
        self.surface_phase = self.rng.next_f32() * std::f32::consts::TAU;
        self.pick_weather();
        for _ in 0..3 {
            let x = self.rng.next_f32() * cols as f32;
            let y = self.rng.next_f32() * rows as f32;
            let strength = 0.5 + self.rng.next_f32() * 0.4;
            types::spawn_ring(
                &mut self.rings,
                self.weather,
                self.on_battery,
                self.quality_scale,
                x,
                y,
                strength,
                cols,
                rows,
            );
            if let Some(r) = self.rings.last_mut() {
                r.r = self.rng.next_f32() * r.max_r * 0.5;
                r.life = 0.4 + self.rng.next_f32() * 0.5;
                r.age = 0.5;
            }
        }
    }

    fn update_frame_time(&mut self, dt: Duration) {
        let dt_secs = dt.as_secs_f32();
        self.frame_time_ema = self.frame_time_ema * 0.9 + dt_secs.min(0.2) * 0.1;
        if self.time > 1.5 {
            let d = dt_secs * if self.on_battery { 0.65 } else { 1.0 };
            if self.frame_time_ema > self.target_frame_time * 1.15 {
                self.quality_scale = (self.quality_scale - 0.12 * d).max(0.3);
            } else if self.frame_time_ema < self.target_frame_time * 1.05 {
                self.quality_scale = (self.quality_scale + 0.04 * d).min(1.0);
            }
        }
    }

    fn update(&mut self, dt: Duration, cols: usize, rows: usize) {
        let dt_secs = dt.as_secs_f32().min(0.1);
        let speed = if self.on_battery { 0.8 } else { 1.0 };
        let delta = dt_secs * speed;
        self.time += delta;
        self.surface_phase += delta * 0.7;
        if self.intro_fade < 1.0 {
            self.intro_fade = (self.intro_fade + delta / 0.45).min(1.0);
        }
        if cols != self.last_cols || rows != self.last_rows {
            self.init(cols, rows);
            return;
        }

        self.sys_timer += delta;
        if self.sys_timer >= 1.5 {
            self.sys_timer = 0.0;
            let sys = get_system_info();
            self.on_battery = sys.power_status.contains("Battery");
            self.accent = query_current_palette().accent;
            self.logo_text = sys.logo_text;
        }

        self.weather_timer -= delta;
        if self.weather_timer <= 0.0 {
            self.pick_weather();
        }

        let on_battery = self.on_battery;
        let quality_scale = self.quality_scale;

        let audio = idle_api::audio::query_audio_bands();
        let audio_energy = audio[0] * 1.5 + audio[1] * 0.8 + audio[2] * 0.4;
        if audio_energy > 0.35 && self.rng.next_f32() < audio_energy.min(0.85) {
            let x = self.rng.next_f32() * cols as f32;
            let y = self.rng.next_f32() * rows as f32;
            let strength = (audio_energy * 0.7).clamp(0.4, 1.2);
            types::spawn_ring(
                &mut self.rings,
                self.weather,
                on_battery,
                quality_scale,
                x,
                y,
                strength,
                cols,
                rows,
            );
        }

        let state = super::update::RippleState {
            rings: &mut self.rings,
            drops: &mut self.drops,
            splashes: &mut self.splashes,
            delayed: &mut self.delayed,
            wind: &mut self.wind,
            weather: &mut self.weather,
            weather_intensity: &mut self.weather_intensity,
            rain_timer: &mut self.rain_timer,
        };

        super::update::update_simulation(
            state,
            &mut self.rng,
            cols,
            rows,
            quality_scale,
            on_battery,
            delta,
        );
    }

    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        super::draw::draw_impl(
            grid,
            cols,
            rows,
            &self.rings,
            &self.drops,
            &self.splashes,
            self.intro_fade,
            self.surface_phase,
            self.weather_intensity,
            self.wind,
            self.accent,
            &self.logo_text,
            &self.accum,
            &self.accent_a,
        );
    }
}
