use crate::storm::Storm;
use crate::storm::types::{AnimalState, AnimalType, BirdState, Splash};

impl Storm {
    pub fn update_lightning(&mut self, delta: f32, cols: usize, rows: usize) {
        self.lightning_timer += delta;
        if self.lightning_flash > 0.0 {
            // Soft recovery: ease flash down (not a hard cut)
            let decay = if self.lightning_flash > 0.12 {
                delta * 2.8 // bright peak short
            } else {
                delta * 1.1 // long soft tail
            };
            self.lightning_flash = (self.lightning_flash - decay).max(0.0);
            if self.lightning_flash <= 0.0 {
                self.lightning_bolts.clear();
            }
        }

        if self.lightning_delay > 0.0 {
            self.lightning_delay -= delta;
            if self.lightning_delay <= 0.0 {
                // Peak flash rarer/softer; most strikes are mid intensity with long tail
                let peak = if self.rng.next_bool(0.18) {
                    0.32 // rare max
                } else {
                    0.16 + self.rng.next_f32() * 0.08
                };
                self.lightning_flash = peak;
                self.subtitle = "[CRACK! Lightning Strike]".to_string();
                self.subtitle_timer = 1.5;

                let target_type = self.rng.next_range(0.0, 6.0) as usize;
                let mut target_x =
                    self.rng.next_range(5.0, (cols.saturating_sub(5)) as f32) as usize;
                let mut target_y = rows.saturating_sub(1);
                let mut hit_bird = false;
                let mut is_bg = false;

                match target_type {
                    0 => {
                        let active_count = self.logo_cells.iter().filter(|c| c.active).count();
                        if active_count > 0 {
                            let pick = self.rng.next_usize(active_count);
                            let mut seen = 0usize;
                            for cell in &self.logo_cells {
                                if !cell.active {
                                    continue;
                                }
                                if seen == pick {
                                    target_x = cell.x;
                                    target_y = cell.y;
                                    break;
                                }
                                seen += 1;
                            }
                        }
                    }
                    1 => {
                        if !self.fg_scenery.is_empty() {
                            let selected =
                                &self.fg_scenery[self.rng.next_usize(self.fg_scenery.len())];
                            target_x = selected.0;
                            target_y = selected.1;
                        }
                    }
                    2 => {
                        if !self.bg_cells.is_empty() {
                            let selected = &self.bg_cells[self.rng.next_usize(self.bg_cells.len())];
                            target_x = selected.0;
                            target_y = selected.1;
                            is_bg = true;
                        }
                    }
                    3 => {
                        if self.bird_state == BirdState::Sitting
                            || self.bird_state == BirdState::Flying
                            || self.bird_state == BirdState::Scared
                        {
                            target_x = self.bird_x as usize;
                            target_y = self.bird_y as usize;
                            hit_bird = true;
                        }
                    }
                    4 => {
                        target_x = self.rng.next_range(0.0, cols as f32) as usize;
                        target_y =
                            self.rng.next_range(rows as f32 * 0.2, rows as f32 * 0.6) as usize;
                        is_bg = true;
                    }
                    _ => {}
                }
                self.lightning_is_background = is_bg;

                let mut curr_x = self.rng.next_range(5.0, (cols.saturating_sub(5)) as f32) as usize;
                let mut bolts = Vec::new();
                let mut main_bolt = Vec::new();
                main_bolt.push((curr_x, 0));

                target_x = target_x.clamp(0, cols.saturating_sub(1));
                target_y = target_y.clamp(0, rows.saturating_sub(1));

                for y in 1..=target_y {
                    let diff = target_x as i32 - curr_x as i32;
                    let step = diff.signum();
                    let drift = if diff.abs() <= 1 {
                        self.rng.next_range(-2.0, 2.0) as i32
                    } else {
                        step + self.rng.next_range(-1.5, 1.5) as i32
                    };
                    curr_x = (curr_x as i32 + drift).clamp(0, cols as i32 - 1) as usize;
                    main_bolt.push((curr_x, y));

                    if y < target_y && self.rng.next_bool(0.15) && bolts.len() < 3 {
                        let mut branch = Vec::new();
                        let mut b_x = curr_x;
                        let b_direction = if self.rng.next_bool(0.5) { 1 } else { -1 };
                        for b_y in y..=(y + self.rng.next_range(4.0, 9.0) as usize).min(target_y) {
                            let b_drift = b_direction * (self.rng.next_range(0.0, 2.0) as i32)
                                + self.rng.next_range(-1.0, 1.0) as i32;
                            b_x = (b_x as i32 + b_drift).clamp(0, cols as i32 - 1) as usize;
                            branch.push((b_x, b_y));
                        }
                        bolts.push(branch);
                    }
                }
                bolts.push(main_bolt);
                self.lightning_bolts = bolts;

                let is_lightning_bg = self.lightning_is_background;
                if hit_bird {
                    self.bird_state = BirdState::Explores;
                    self.bird_timer = 2.5;
                    self.bird_vx = self.rng.next_range(-20.0, 20.0);
                    self.bird_vy = self.rng.next_range(-15.0, -5.0);
                    self.subtitle = "[Subtitles: Bird electrified by lightning surge!]".to_string();
                    self.subtitle_timer = 2.0;

                    for _ in 0..10 {
                        self.splashes.push(Splash {
                            x: self.bird_x,
                            y: self.bird_y,
                            vx: self.rng.next_range(-10.0, 10.0),
                            vy: self.rng.next_range(-10.0, 10.0),
                            life: self.rng.next_range(0.4, 0.8),
                            color: (255, 255, 255),
                            is_background: false,
                        });
                    }
                } else {
                    let spark_count = if target_y == rows.saturating_sub(1) {
                        10
                    } else {
                        16
                    };
                    for _ in 0..spark_count {
                        self.splashes.push(Splash {
                            x: target_x as f32,
                            y: target_y as f32,
                            vx: self.rng.next_range(-14.0, 14.0),
                            vy: self.rng.next_range(-12.0, 1.0),
                            life: self.rng.next_range(0.4, 0.8),
                            color: (255, 235, 140),
                            is_background: is_lightning_bg,
                        });
                    }

                    let mut hit_logo_cell = false;
                    for cell in &mut self.logo_cells {
                        if cell.x == target_x && cell.y == target_y {
                            cell.glow = 1.0;
                            cell.water = (cell.water + 0.8).min(2.5);
                            hit_logo_cell = true;
                        }
                    }
                    if hit_logo_cell {
                        self.subtitle = "[Subtitles: System Surge Detected! *BZZZT*]".to_string();
                        self.subtitle_timer = 2.0;
                    }
                }

                if let Some(ref mut animal) = self.active_animal
                    && animal.state != AnimalState::Startled
                    && animal.state != AnimalState::WalkingOff
                {
                    if animal.animal_type == AnimalType::Bigfoot {
                        self.subtitle =
                            "[Subtitles: Bigfoot watches the lightning calmly]".to_string();
                        self.subtitle_timer = 2.2;
                    } else {
                        animal.state = AnimalState::Startled;
                        animal.timer = 0.8;
                        self.subtitle = match animal.animal_type {
                            AnimalType::Deer => {
                                "[Subtitles: Deer startled by the blast!]".to_string()
                            }
                            AnimalType::Bear => {
                                "[Subtitles: Bear startled! *Growls angrily*]".to_string()
                            }
                            _ => String::new(),
                        };
                        self.subtitle_timer = 2.0;
                    }
                }

                if !hit_bird
                    && (self.bird_state == BirdState::Sitting
                        || self.bird_state == BirdState::Flying)
                {
                    self.bird_state = BirdState::Scared;
                    self.bird_timer = 0.6;
                }
            }
        }

        // Rarer strikes overall; longer quiet periods between rumbles
        if self.lightning_timer > 10.0 && self.rng.next_bool(0.035) && self.lightning_delay <= 0.0 {
            self.lightning_timer = 0.0;
            self.lightning_delay = 0.9 + self.rng.next_f32() * 0.6;
            self.subtitle = "[Distant thunder rumbling...]".to_string();
            self.subtitle_timer = 1.0;
        }
    }
}
