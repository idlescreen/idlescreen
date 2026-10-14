// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Deterministic LCG random number generator, HSL/RGB math, and palette primitives.

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
    a + (b - a) * factor.clamp(0.0, 1.0)
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
