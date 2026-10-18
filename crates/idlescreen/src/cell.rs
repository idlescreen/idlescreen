// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Terminal cell definitions and LCG pseudo-random number generator.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

static RNG_COUNTER: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

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

#[derive(Clone, Debug)]
pub struct LcgRng(u64);

impl LcgRng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn new_random() -> Self {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        let seq = RNG_COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
        let pid = u64::from(std::process::id()).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let mut z = nanos ^ seq ^ pid;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let seed = z ^ (z >> 31);
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
            ((self.next_u64() >> 32) % max as u64) as usize
        }
    }

    pub fn next_bool(&mut self, prob: f32) -> bool {
        self.next_f32() < prob
    }
}
