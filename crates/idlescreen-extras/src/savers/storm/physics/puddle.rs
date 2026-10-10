//! Splash physics, logo cell decay, and puddle evaporation for Storm.

use crate::storm::Storm;

impl Storm {
    pub(crate) fn update_splashes_and_puddles(&mut self, delta: f32, cols: usize) {
        for s in &mut self.splashes {
            s.x += s.vx * delta;
            s.y += s.vy * delta;
            s.vx += self.wind * delta * 0.25;
            s.vy += 9.8 * delta;
            s.life -= delta;
        }
        self.splashes.retain(|s| s.life > 0.0);

        for cell in &mut self.logo_cells {
            if cell.glow > 0.0 {
                cell.glow -= delta * 1.5;
            }
            if cell.water > 0.0 {
                cell.water -= delta * 0.45;
                if cell.water < 0.0 {
                    cell.water = 0.0;
                }
            }
        }

        for x in 0..cols {
            if x < self.puddle.len() && self.puddle[x] > 0.0 {
                self.puddle[x] -= delta * 0.28;
                if self.puddle[x] < 0.0 {
                    self.puddle[x] = 0.0;
                }
            }
        }
    }
}
