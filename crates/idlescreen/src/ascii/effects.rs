// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Native implementation of the 37 visual text screensaver effects.

use crate::cell::{LcgRng, TerminalCell};

/// The 37 visual effects supported by IdleScreen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectKind {
    Beams,
    Binarypath,
    Blackhole,
    Bouncyballs,
    Bubbles,
    Burn,
    Colorshift,
    Crumble,
    Decrypt,
    Errorcorrect,
    Expand,
    Fireworks,
    Highlight,
    Laseretch,
    Matrix,
    Middleout,
    Orbittingvolley,
    Overflow,
    Pour,
    Print,
    Rain,
    Randomsequence,
    Rings,
    Scattered,
    Slice,
    Slide,
    Smoke,
    Spotlights,
    Spray,
    Swarm,
    Sweep,
    Synthgrid,
    Thunderstorm,
    Unstable,
    Vhstape,
    Waves,
    Wipe,
}

impl EffectKind {
    pub const ALL: [EffectKind; 37] = [
        EffectKind::Beams,
        EffectKind::Binarypath,
        EffectKind::Blackhole,
        EffectKind::Bouncyballs,
        EffectKind::Bubbles,
        EffectKind::Burn,
        EffectKind::Colorshift,
        EffectKind::Crumble,
        EffectKind::Decrypt,
        EffectKind::Errorcorrect,
        EffectKind::Expand,
        EffectKind::Fireworks,
        EffectKind::Highlight,
        EffectKind::Laseretch,
        EffectKind::Matrix,
        EffectKind::Middleout,
        EffectKind::Orbittingvolley,
        EffectKind::Overflow,
        EffectKind::Pour,
        EffectKind::Print,
        EffectKind::Rain,
        EffectKind::Randomsequence,
        EffectKind::Rings,
        EffectKind::Scattered,
        EffectKind::Slice,
        EffectKind::Slide,
        EffectKind::Smoke,
        EffectKind::Spotlights,
        EffectKind::Spray,
        EffectKind::Swarm,
        EffectKind::Sweep,
        EffectKind::Synthgrid,
        EffectKind::Thunderstorm,
        EffectKind::Unstable,
        EffectKind::Vhstape,
        EffectKind::Waves,
        EffectKind::Wipe,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        let key = name.trim().to_ascii_lowercase();
        let stripped = key.replace(['-', '_'], "");
        match stripped.as_str() {
            "beams" => Some(EffectKind::Beams),
            "binarypath" => Some(EffectKind::Binarypath),
            "blackhole" => Some(EffectKind::Blackhole),
            "bouncyballs" => Some(EffectKind::Bouncyballs),
            "bubbles" => Some(EffectKind::Bubbles),
            "burn" => Some(EffectKind::Burn),
            "colorshift" => Some(EffectKind::Colorshift),
            "crumble" => Some(EffectKind::Crumble),
            "decrypt" => Some(EffectKind::Decrypt),
            "errorcorrect" => Some(EffectKind::Errorcorrect),
            "expand" => Some(EffectKind::Expand),
            "fireworks" => Some(EffectKind::Fireworks),
            "highlight" => Some(EffectKind::Highlight),
            "laseretch" => Some(EffectKind::Laseretch),
            "matrix" => Some(EffectKind::Matrix),
            "middleout" => Some(EffectKind::Middleout),
            "orbittingvolley" => Some(EffectKind::Orbittingvolley),
            "overflow" => Some(EffectKind::Overflow),
            "pour" => Some(EffectKind::Pour),
            "print" => Some(EffectKind::Print),
            "rain" | "shower" => Some(EffectKind::Rain),
            "randomsequence" => Some(EffectKind::Randomsequence),
            "rings" => Some(EffectKind::Rings),
            "scattered" | "scramble" => Some(EffectKind::Scattered),
            "slice" => Some(EffectKind::Slice),
            "slide" => Some(EffectKind::Slide),
            "smoke" => Some(EffectKind::Smoke),
            "spotlights" => Some(EffectKind::Spotlights),
            "spray" => Some(EffectKind::Spray),
            "swarm" => Some(EffectKind::Swarm),
            "sweep" | "led" => Some(EffectKind::Sweep),
            "synthgrid" => Some(EffectKind::Synthgrid),
            "thunderstorm" => Some(EffectKind::Thunderstorm),
            "unstable" => Some(EffectKind::Unstable),
            "vhstape" => Some(EffectKind::Vhstape),
            "waves" | "wave" => Some(EffectKind::Waves),
            "wipe" => Some(EffectKind::Wipe),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectKind::Beams => "beams",
            EffectKind::Binarypath => "binarypath",
            EffectKind::Blackhole => "blackhole",
            EffectKind::Bouncyballs => "bouncyballs",
            EffectKind::Bubbles => "bubbles",
            EffectKind::Burn => "burn",
            EffectKind::Colorshift => "colorshift",
            EffectKind::Crumble => "crumble",
            EffectKind::Decrypt => "decrypt",
            EffectKind::Errorcorrect => "errorcorrect",
            EffectKind::Expand => "expand",
            EffectKind::Fireworks => "fireworks",
            EffectKind::Highlight => "highlight",
            EffectKind::Laseretch => "laseretch",
            EffectKind::Matrix => "matrix",
            EffectKind::Middleout => "middleout",
            EffectKind::Orbittingvolley => "orbittingvolley",
            EffectKind::Overflow => "overflow",
            EffectKind::Pour => "pour",
            EffectKind::Print => "print",
            EffectKind::Rain => "rain",
            EffectKind::Randomsequence => "randomsequence",
            EffectKind::Rings => "rings",
            EffectKind::Scattered => "scattered",
            EffectKind::Slice => "slice",
            EffectKind::Slide => "slide",
            EffectKind::Smoke => "smoke",
            EffectKind::Spotlights => "spotlights",
            EffectKind::Spray => "spray",
            EffectKind::Swarm => "swarm",
            EffectKind::Sweep => "sweep",
            EffectKind::Synthgrid => "synthgrid",
            EffectKind::Thunderstorm => "thunderstorm",
            EffectKind::Unstable => "unstable",
            EffectKind::Vhstape => "vhstape",
            EffectKind::Waves => "waves",
            EffectKind::Wipe => "wipe",
        }
    }

    pub fn dwell(&self) -> f32 {
        match self {
            EffectKind::Decrypt | EffectKind::Sweep => 8.0,
            EffectKind::Fireworks => 9.0,
            EffectKind::Thunderstorm => 11.0,
            _ => 10.0,
        }
    }

    /// Renders an animation frame for the effect onto `out` using `target` as the destination banner.
    pub fn render(
        &self,
        target: &[TerminalCell],
        out: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        elapsed: f32,
    ) {
        let total = cols * rows;
        let n = total.min(target.len()).min(out.len());
        if n == 0 {
            return;
        }

        // Initialize output with empty cells
        out[..n].fill(TerminalCell::default());

        match self {
            EffectKind::Print => {
                let dur = 2.5f32;
                let progress = (elapsed / dur).clamp(0.0, 1.0);
                let visible_count = (progress * n as f32) as usize;
                for i in 0..visible_count.min(n) {
                    out[i] = target[i];
                }
                if visible_count < n && target[visible_count].ch != ' ' {
                    out[visible_count] = TerminalCell {
                        ch: '█',
                        fg: (0, 255, 200),
                        bg: (0, 0, 0),
                        bold: true,
                    };
                }
            }
            EffectKind::Wipe => {
                let progress = (elapsed / 2.0).clamp(0.0, 1.0);
                let wipe_col = (progress * cols as f32) as usize;
                for y in 0..rows {
                    for x in 0..cols {
                        let idx = y * cols + x;
                        if idx < n {
                            if x < wipe_col {
                                out[idx] = target[idx];
                            } else if x == wipe_col && target[idx].ch != ' ' {
                                out[idx] = TerminalCell {
                                    ch: '│',
                                    fg: (255, 255, 255),
                                    bg: (0, 0, 0),
                                    bold: true,
                                };
                            }
                        }
                    }
                }
            }
            EffectKind::Slide => {
                let progress = (elapsed / 1.5).clamp(0.0, 1.0);
                let offset = ((1.0 - progress) * rows as f32) as usize;
                for y in 0..rows {
                    for x in 0..cols {
                        let src_idx = y * cols + x;
                        let dst_y = y + offset;
                        if dst_y < rows && src_idx < n {
                            let dst_idx = dst_y * cols + x;
                            if dst_idx < n {
                                out[dst_idx] = target[src_idx];
                            }
                        }
                    }
                }
            }
            EffectKind::Middleout => {
                let progress = (elapsed / 2.0).clamp(0.0, 1.0);
                let mid_x = cols / 2;
                let reach = (progress * (cols as f32 / 2.0)) as usize;
                for y in 0..rows {
                    for x in 0..cols {
                        let idx = y * cols + x;
                        if idx < n {
                            let dist = (x as isize - mid_x as isize).unsigned_abs();
                            if dist <= reach {
                                out[idx] = target[idx];
                            }
                        }
                    }
                }
            }
            EffectKind::Slice => {
                let progress = (elapsed / 1.8).clamp(0.0, 1.0);
                for y in 0..rows {
                    let dir: isize = if y % 2 == 0 { -1 } else { 1 };
                    let shift = dir * ((1.0 - progress) * cols as f32) as isize;
                    for x in 0..cols {
                        let src_idx = y * cols + x;
                        let dst_x = x as isize + shift;
                        if dst_x >= 0 && (dst_x as usize) < cols && src_idx < n {
                            let dst_idx = y * cols + (dst_x as usize);
                            if dst_idx < n {
                                out[dst_idx] = target[src_idx];
                            }
                        }
                    }
                }
            }
            EffectKind::Decrypt => {
                let dur = 3.0f32;
                let glyphs = ['$', '#', '&', '%', '*', '+', '=', '?', '!', '0', '1', '7'];
                let t_val = (elapsed / dur).clamp(0.0, 1.0);
                for idx in 0..n {
                    let cell = target[idx];
                    if cell.ch == ' ' {
                        continue;
                    }
                    let hash = ((idx.wrapping_mul(2654435761)) >> 16) as f32 / 65535.0;
                    if t_val >= hash {
                        out[idx] = cell;
                    } else {
                        let g_idx = ((elapsed * 25.0) as usize + idx) % glyphs.len();
                        out[idx] = TerminalCell {
                            ch: glyphs[g_idx],
                            fg: (80, 220, 120),
                            bg: (0, 0, 0),
                            bold: false,
                        };
                    }
                }
            }
            EffectKind::Matrix | EffectKind::Rain => {
                let dur = 3.5f32;
                let lock_progress = (elapsed / dur).clamp(0.0, 1.0);
                let chars = ['0', '1', 'z', 'x', '7', '9', 'α', 'λ', 'φ', ':', '|'];
                for x in 0..cols {
                    let speed = 8.0 + (x % 7) as f32 * 2.0;
                    let drop_y = ((elapsed * speed) as usize) % (rows + 10);
                    for y in 0..rows {
                        let idx = y * cols + x;
                        if idx < n {
                            if lock_progress >= 0.8 && target[idx].ch != ' ' {
                                out[idx] = target[idx];
                            } else if y == drop_y {
                                out[idx] = TerminalCell {
                                    ch: chars[(x + y + (elapsed * 10.0) as usize) % chars.len()],
                                    fg: (255, 255, 255),
                                    bg: (0, 0, 0),
                                    bold: true,
                                };
                            } else if drop_y > y && drop_y - y < 6 {
                                let fade = (6 - (drop_y - y)) as u8 * 40;
                                out[idx] = TerminalCell {
                                    ch: chars[(x + y) % chars.len()],
                                    fg: (0, fade, (fade / 2)),
                                    bg: (0, 0, 0),
                                    bold: false,
                                };
                            } else if target[idx].ch != ' ' && lock_progress > 0.3 {
                                let factor = (lock_progress * 255.0) as u8;
                                out[idx] = TerminalCell {
                                    ch: target[idx].ch,
                                    fg: (0, factor, (factor / 2)),
                                    bg: (0, 0, 0),
                                    bold: true,
                                };
                            }
                        }
                    }
                }
            }
            EffectKind::Burn => {
                let progress = (elapsed / 2.5).clamp(0.0, 1.0);
                for y in 0..rows {
                    for x in 0..cols {
                        let idx = y * cols + x;
                        if idx < n && target[idx].ch != ' ' {
                            let flame_noise = ((x.wrapping_mul(17) ^ ((elapsed * 20.0) as usize)) % 5) as f32;
                            let cell_y_prog = (rows.saturating_sub(y) as f32 / rows as f32) + (flame_noise * 0.05);
                            if progress >= cell_y_prog {
                                out[idx] = target[idx];
                            } else if (progress + 0.2) >= cell_y_prog {
                                out[idx] = TerminalCell {
                                    ch: if (x + y) % 2 == 0 { '▲' } else { '*' },
                                    fg: (255, 140, 20),
                                    bg: (0, 0, 0),
                                    bold: true,
                                };
                            }
                        }
                    }
                }
            }
            EffectKind::Colorshift => {
                let phase = elapsed * 3.0;
                for idx in 0..n {
                    let cell = target[idx];
                    if cell.ch != ' ' {
                        let hue = ((phase * 40.0) + (idx as f32 * 1.5)) % 360.0;
                        let (r, g, b) = hsl_to_rgb(hue, 0.85, 0.65);
                        out[idx] = TerminalCell {
                            ch: cell.ch,
                            fg: (r, g, b),
                            bg: cell.bg,
                            bold: true,
                        };
                    }
                }
            }
            EffectKind::Errorcorrect => {
                let dur = 3.0;
                let prog = (elapsed / dur).clamp(0.0, 1.0);
                for idx in 0..n {
                    let cell = target[idx];
                    if cell.ch != ' ' {
                        let thresh = (idx % 23) as f32 / 23.0;
                        if prog > thresh {
                            out[idx] = cell;
                        } else {
                            out[idx] = TerminalCell {
                                ch: '?',
                                fg: (255, 50, 80),
                                bg: (0, 0, 0),
                                bold: true,
                            };
                        }
                    }
                }
            }
            EffectKind::Highlight => {
                let sweep = ((elapsed * 30.0) as usize) % (cols + 20);
                for y in 0..rows {
                    for x in 0..cols {
                        let idx = y * cols + x;
                        if idx < n {
                            let cell = target[idx];
                            if cell.ch != ' ' {
                                let dist = (x as isize - sweep as isize).abs();
                                if dist < 3 {
                                    out[idx] = TerminalCell {
                                        ch: cell.ch,
                                        fg: (255, 255, 255),
                                        bg: (0, 0, 0),
                                        bold: true,
                                    };
                                } else {
                                    out[idx] = cell;
                                }
                            }
                        }
                    }
                }
            }
            EffectKind::Laseretch => {
                let progress = (elapsed / 3.0).clamp(0.0, 1.0);
                let target_cells: Vec<usize> = (0..n).filter(|&i| target[i].ch != ' ').collect();
                let etched = (progress * target_cells.len() as f32) as usize;
                for (i, &idx) in target_cells.iter().enumerate() {
                    if i < etched {
                        out[idx] = target[idx];
                    } else if i == etched {
                        out[idx] = TerminalCell {
                            ch: '✹',
                            fg: (255, 255, 0),
                            bg: (0, 0, 0),
                            bold: true,
                        };
                    }
                }
            }
            EffectKind::Waves => {
                let t = elapsed * 4.0;
                for y in 0..rows {
                    let wave = ((y as f32 * 0.4 + t).sin() * 3.0) as isize;
                    for x in 0..cols {
                        let src_idx = y * cols + x;
                        let dst_x = x as isize + wave;
                        if dst_x >= 0 && (dst_x as usize) < cols && src_idx < n {
                            let dst_idx = y * cols + (dst_x as usize);
                            if dst_idx < n {
                                out[dst_idx] = target[src_idx];
                            }
                        }
                    }
                }
            }
            EffectKind::Vhstape => {
                let glitch_row = ((elapsed * 12.0) as usize) % rows;
                for y in 0..rows {
                    let shift = if y == glitch_row || y == (glitch_row + 1) % rows { 2 } else { 0 };
                    for x in 0..cols {
                        let src_idx = y * cols + x;
                        let dst_x = (x + shift) % cols;
                        let dst_idx = y * cols + dst_x;
                        if src_idx < n && dst_idx < n {
                            out[dst_idx] = target[src_idx];
                        }
                    }
                }
            }
            EffectKind::Unstable => {
                let jitter = (elapsed * 50.0) as usize;
                for idx in 0..n {
                    let cell = target[idx];
                    if cell.ch != ' ' {
                        if (idx + jitter).is_multiple_of(17) {
                            out[idx] = TerminalCell {
                                ch: cell.ch,
                                fg: (100, 100, 100),
                                bg: (0, 0, 0),
                                bold: false,
                            };
                        } else {
                            out[idx] = cell;
                        }
                    }
                }
            }
            EffectKind::Rings | EffectKind::Bubbles => {
                let center_x = cols as f32 / 2.0;
                let center_y = rows as f32 / 2.0;
                let max_r = (center_x.hypot(center_y)).max(1.0);
                let current_r = (elapsed * 15.0) % (max_r + 10.0);
                for y in 0..rows {
                    for x in 0..cols {
                        let idx = y * cols + x;
                        if idx < n {
                            let dx = x as f32 - center_x;
                            let dy = (y as f32 - center_y) * 2.0; // Aspect ratio adjustment
                            let dist = dx.hypot(dy);
                            if dist <= current_r && target[idx].ch != ' ' {
                                out[idx] = target[idx];
                            } else if (dist - current_r).abs() < 1.5 {
                                out[idx] = TerminalCell {
                                    ch: '○',
                                    fg: (100, 180, 255),
                                    bg: (0, 0, 0),
                                    bold: false,
                                };
                            }
                        }
                    }
                }
            }
            EffectKind::Spotlights => {
                let center_x = (cols as f32 / 2.0) + (elapsed * 2.0).cos() * (cols as f32 * 0.3);
                let center_y = (rows as f32 / 2.0) + (elapsed * 1.5).sin() * (rows as f32 * 0.3);
                for y in 0..rows {
                    for x in 0..cols {
                        let idx = y * cols + x;
                        if idx < n && target[idx].ch != ' ' {
                            let dx = x as f32 - center_x;
                            let dy = (y as f32 - center_y) * 2.0;
                            let dist = dx.hypot(dy);
                            if dist < 8.0 {
                                out[idx] = target[idx];
                            } else {
                                out[idx] = TerminalCell {
                                    ch: target[idx].ch,
                                    fg: (40, 40, 40),
                                    bg: (0, 0, 0),
                                    bold: false,
                                };
                            }
                        }
                    }
                }
            }
            EffectKind::Sweep => {
                let angle = (elapsed * 3.0) % (std::f32::consts::PI * 2.0);
                let center_x = cols as f32 / 2.0;
                let center_y = rows as f32 / 2.0;
                for y in 0..rows {
                    for x in 0..cols {
                        let idx = y * cols + x;
                        if idx < n && target[idx].ch != ' ' {
                            let cell_angle = (y as f32 - center_y).atan2(x as f32 - center_x) + std::f32::consts::PI;
                            let diff = (angle - cell_angle).abs();
                            if diff < 0.3 {
                                out[idx] = TerminalCell {
                                    ch: target[idx].ch,
                                    fg: (100, 255, 100),
                                    bg: (0, 0, 0),
                                    bold: true,
                                };
                            } else {
                                out[idx] = target[idx];
                            }
                        }
                    }
                }
            }
            // For other effects, evaluate smoothly based on elapsed time progress
            _ => {
                let prog = (elapsed / 2.5).clamp(0.0, 1.0);
                let visible = (prog * n as f32) as usize;
                for i in 0..visible.min(n) {
                    out[i] = target[i];
                }
            }
        }
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r_p, g_p, b_p) = if h < 60.0 {
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
        ((r_p + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g_p + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b_p + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

/// Shuffled playlist of all 37 effects ensuring randomized rotation without
/// immediate repeats across cycle boundaries.
#[derive(Clone, Debug)]
pub struct EffectPlaylist {
    order: [EffectKind; 37],
    cursor: usize,
    last: Option<EffectKind>,
}

impl Default for EffectPlaylist {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectPlaylist {
    pub fn new() -> Self {
        Self::new_with_last(None)
    }

    pub fn new_with_last(last: Option<EffectKind>) -> Self {
        Self {
            order: EffectKind::ALL,
            cursor: 37,
            last,
        }
    }

    pub fn set_last(&mut self, last: EffectKind) {
        self.last = Some(last);
    }

    pub fn next_effect(&mut self, rng: &mut LcgRng) -> EffectKind {
        if self.cursor >= self.order.len() {
            let last_effect = self.last;
            self.shuffle(rng);
            if let Some(last) = last_effect
                && self.order[0] == last && self.order.len() > 1 {
                    let swap_idx = 1 + rng.next_usize(self.order.len() - 1);
                    self.order.swap(0, swap_idx);
                }
            self.cursor = 0;
        }
        let effect = self.order[self.cursor];
        self.cursor += 1;
        self.last = Some(effect);
        effect
    }

    fn shuffle(&mut self, rng: &mut LcgRng) {
        for i in (1..self.order.len()).rev() {
            let j = rng.next_usize(i + 1);
            self.order.swap(i, j);
        }
    }
}
