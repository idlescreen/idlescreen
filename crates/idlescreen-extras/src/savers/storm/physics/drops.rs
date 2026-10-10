//! Rain drop updates, collision handling, and splash/puddle updates.
//! Math: dy += gravity*dt, wind effect, bounce on ground with 0.3 factor, random splash.
//! Precision: f32 sufficient for screen coords; fixed dt in caller for determinism.
//! Part of screensavers for trance-daemon idle on Pop!_OS.

use crate::storm::Storm;
use crate::storm::types::{Drop, Phase, Splash};

impl Storm {
    /// 0 = far, 1 = mid, 2 = near
    fn pick_rain_layer(rng: &mut crate::runner::LcgRng) -> u8 {
        let r = rng.next_f32();
        if r < 0.35 {
            0
        } else if r < 0.70 {
            1
        } else {
            2
        }
    }

    fn rain_layer_props(
        rng: &mut crate::runner::LcgRng,
        layer: u8,
        speed_mult: f32,
    ) -> ((u8, u8, u8), f32) {
        let mut color = Self::cold_rain_color(rng);
        let (dim, speed_lo, speed_hi) = match layer {
            0 => (0.32, 14.0, 24.0), // far: slow + dim
            1 => (0.65, 22.0, 36.0), // mid
            _ => (1.0, 32.0, 52.0),  // near: fast + bright
        };
        color = (
            (color.0 as f32 * dim) as u8,
            (color.1 as f32 * dim) as u8,
            (color.2 as f32 * dim) as u8,
        );
        let vy = rng.next_range(speed_lo, speed_hi) * speed_mult;
        (color, vy)
    }

    pub fn update_drops(&mut self, delta: f32, cols: usize, rows: usize, speed_mult: f32) {
        let mut target_drops = match self.drop_count_opt {
            0 => (cols).clamp(20, 150),
            2 => (cols * 3).clamp(60, 600),
            _ => (cols * 2).clamp(40, 400),
        };
        if self.on_battery {
            target_drops = (target_drops as f32 * 0.55) as usize;
        }
        target_drops = (target_drops as f32 * self.quality_scale) as usize;

        if self.drops.len() < target_drops {
            while self.drops.len() < target_drops {
                let x = if self.phase == Phase::Building && self.rng.next_bool(0.6) {
                    let inactive_count = self.logo_cells.iter().filter(|c| !c.active).count();
                    if inactive_count > 0 {
                        let pick = self.rng.next_usize(inactive_count);
                        let mut seen = 0usize;
                        let mut x_pos = self.rng.next_range(0.0, cols as f32);
                        for cell in &self.logo_cells {
                            if cell.active {
                                continue;
                            }
                            if seen == pick {
                                x_pos = cell.x as f32;
                                break;
                            }
                            seen += 1;
                        }
                        x_pos
                    } else {
                        self.rng.next_range(0.0, cols as f32)
                    }
                } else {
                    self.rng.next_range(0.0, cols as f32)
                };

                let layer = Self::pick_rain_layer(&mut self.rng);
                let is_bg = layer == 0;
                let (color, vy) = Self::rain_layer_props(&mut self.rng, layer, speed_mult);

                self.drops.push(Drop {
                    x,
                    y: -self.rng.next_range(1.0, rows as f32),
                    vy,
                    color,
                    is_background: is_bg,
                    layer,
                });
            }
        } else if self.drops.len() > target_drops {
            self.drops.truncate(target_drops);
        }

        // Update drops position & collisions
        let mut drops = std::mem::take(&mut self.drops);
        for drop in &mut drops {
            drop.y += drop.vy * delta;

            // Wind drifts the drop horizontally
            drop.x += self.wind * delta;
            // Wrap horizontally around the screen columns
            if drop.x < 0.0 {
                drop.x += cols as f32;
            } else if drop.x >= cols as f32 {
                drop.x -= cols as f32;
            }

            let col = drop.x as usize;
            if drop.y >= 0.0 {
                let row = drop.y as usize;

                if col < cols && row < rows {
                    // Background drops do NOT collide with foreground elements
                    if !drop.is_background {
                        // Check if we hit any logo cell (whether active or inactive)
                        let mut hit = false;
                        for cell in &mut self.logo_cells {
                            if cell.x == col && cell.y == row {
                                if !cell.active && self.phase == Phase::Building {
                                    cell.active = true;
                                    cell.glow = 1.0;
                                }

                                if cell.active {
                                    // Rain water piles up on the active OS/Kernel cells
                                    cell.water = (cell.water + 0.45).min(2.5);

                                    // Spawn splash particles
                                    let splash_count = if self.on_battery { 1 } else { 3 };
                                    let splash_count = (splash_count as f32 * self.quality_scale)
                                        .max(1.0)
                                        as usize;
                                    for _ in 0..splash_count {
                                        self.splashes.push(Splash {
                                            x: col as f32,
                                            y: row as f32,
                                            vx: self.rng.next_range(-3.0, 3.0),
                                            vy: self.rng.next_range(-2.0, -0.5),
                                            life: 0.5,
                                            color: drop.color,
                                            is_background: false,
                                        });
                                    }

                                    // Reset drop with new depth layer
                                    let layer = Self::pick_rain_layer(&mut self.rng);
                                    let (color, vy) =
                                        Self::rain_layer_props(&mut self.rng, layer, speed_mult);
                                    drop.is_background = layer == 0;
                                    drop.layer = layer;
                                    drop.color = color;
                                    drop.y = -self.rng.next_range(1.0, rows as f32);
                                    drop.vy = vy;
                                    hit = true;
                                    break;
                                }
                            }
                        }
                        if hit {
                            continue;
                        }
                    }
                }
            }

            // Reset drop if it falls off bottom
            if drop.y >= (rows as f32 - 1.0) && cols > 0 {
                let col = (drop.x as usize).min(cols.saturating_sub(1));

                // Foreground drops spawn floor splash particles and accumulate puddles
                if drop.is_background {
                    // Background splashes
                    for _ in 0..1 {
                        self.splashes.push(Splash {
                            x: col as f32,
                            y: (rows as f32 - 1.0),
                            vx: self.rng.next_range(-2.0, 2.0),
                            vy: self.rng.next_range(-1.5, -0.5),
                            life: self.rng.next_range(0.2, 0.4),
                            color: drop.color,
                            is_background: true,
                        });
                    }
                } else {
                    let splash_count = if self.on_battery { 1 } else { 2 };
                    let splash_count = (splash_count as f32 * self.quality_scale).max(1.0) as usize;
                    for _ in 0..splash_count {
                        self.splashes.push(Splash {
                            x: col as f32,
                            y: (rows as f32 - 1.0),
                            vx: self.rng.next_range(-4.0, 4.0),
                            vy: self.rng.next_range(-3.0, -1.0),
                            life: self.rng.next_range(0.3, 0.6),
                            color: drop.color,
                            is_background: false,
                        });
                    }

                    // Accumulate puddle on the floor
                    if col < self.puddle.len() {
                        self.puddle[col] = (self.puddle[col] + 0.38).min(3.0);
                        let p_col = self.puddle_color[col];
                        let drop_color = drop.color;
                        self.puddle_color[col] = (
                            (p_col.0 as f32 * 0.6 + drop_color.0 as f32 * 0.4) as u8,
                            (p_col.1 as f32 * 0.6 + drop_color.1 as f32 * 0.4) as u8,
                            (p_col.2 as f32 * 0.6 + drop_color.2 as f32 * 0.4) as u8,
                        );
                    }
                }

                let layer = Self::pick_rain_layer(&mut self.rng);
                let (color, vy) = Self::rain_layer_props(&mut self.rng, layer, speed_mult);
                drop.is_background = layer == 0;
                drop.layer = layer;
                drop.color = color;
                drop.y = -self.rng.next_range(1.0, rows as f32);
                drop.vy = vy;
            }
        }
        self.drops = drops;

        self.update_splashes_and_puddles(delta, cols);
    }
}
