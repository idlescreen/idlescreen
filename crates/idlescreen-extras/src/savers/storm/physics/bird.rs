use crate::storm::Storm;
use crate::storm::types::{BirdState, Splash};

impl Storm {
    pub fn update_bird(&mut self, delta: f32, cols: usize, rows: usize) {
        match self.bird_state {
            BirdState::Sitting => {
                self.bird_x = self.bird_perch_x;
                self.bird_y = self.bird_perch_y;
                self.bird_timer -= delta;
                if self.bird_timer <= 0.0 {
                    self.bird_state = BirdState::Flying;
                    self.bird_timer = self.rng.next_range(6.0, 12.0);
                    self.bird_vx = self.rng.next_range(4.0, 8.0);
                    self.bird_vy = self.rng.next_range(-4.0, -1.5);
                    if self.rng.next_bool(0.15) && self.subtitle_timer <= 0.0 {
                        self.subtitle = "[Bird took off]".to_string();
                        self.subtitle_timer = 1.5;
                    }
                }
            }
            BirdState::Flying => {
                self.bird_timer -= delta;
                self.bird_x += self.bird_vx * delta;
                self.bird_y += self.bird_vy * delta;

                if self.rng.next_bool(0.25) {
                    self.bird_wing_flap = !self.bird_wing_flap;
                }

                if self.bird_x >= cols as f32 || self.bird_y < 0.0 || self.bird_timer <= 0.0 {
                    self.bird_state = BirdState::Dead;
                    self.bird_timer = self.rng.next_range(8.0, 18.0);
                }
            }
            BirdState::Scared => {
                self.bird_timer -= delta;
                if self.bird_timer <= 0.0 {
                    self.bird_state = BirdState::Flying;
                    self.bird_timer = 4.0;

                    let current_x = self.bird_x;
                    let current_y = self.bird_y;
                    self.bird_vx = self.rng.next_range(9.0, 15.0);
                    self.bird_vy = self.rng.next_range(-6.0, -3.0);

                    if (current_x - self.bird_perch_x).abs() < 1.0
                        && (current_y - self.bird_perch_y).abs() < 1.0
                    {
                        self.bird_x = self.bird_perch_x + 1.0;
                    }
                }
            }
            BirdState::Explores => {
                self.bird_timer -= delta;
                self.bird_x += self.bird_vx * delta;
                self.bird_y += self.bird_vy * delta;

                if self.rng.next_bool(0.18) {
                    self.bird_vx = self.rng.next_range(-25.0, 25.0);
                    self.bird_vy = self.rng.next_range(-20.0, 20.0);
                }

                if self.bird_x < 1.0 {
                    self.bird_x = 1.0;
                    self.bird_vx = -self.bird_vx * 0.8;
                } else if self.bird_x >= (cols as f32 - 1.0) {
                    self.bird_x = cols as f32 - 2.0;
                    self.bird_vx = -self.bird_vx * 0.8;
                }

                if self.bird_y < 1.0 {
                    self.bird_y = 1.0;
                    self.bird_vy = -self.bird_vy * 0.8;
                } else if self.bird_y >= (rows as f32 - 1.0) {
                    self.bird_y = rows as f32 - 2.0;
                    self.bird_vy = -self.bird_vy * 0.8;
                }

                if self.rng.next_bool(0.25) {
                    self.bird_wing_flap = !self.bird_wing_flap;
                }

                if self.rng.next_bool(0.6) {
                    self.splashes.push(Splash {
                        x: self.bird_x,
                        y: self.bird_y,
                        vx: self.rng.next_range(-4.0, 4.0),
                        vy: self.rng.next_range(-4.0, 4.0),
                        life: self.rng.next_range(0.3, 0.6),
                        color: if self.rng.next_bool(0.5) {
                            (255, 235, 140)
                        } else {
                            (100, 220, 255)
                        },
                        is_background: false,
                    });
                }

                if self.bird_timer <= 0.0 {
                    self.bird_state = BirdState::Dead;
                    self.bird_timer = 8.0;
                    self.subtitle = "[Subtitles: Bird Vaporized!]".to_string();
                    self.subtitle_timer = 2.0;

                    for _ in 0..20 {
                        self.splashes.push(Splash {
                            x: self.bird_x,
                            y: self.bird_y,
                            vx: self.rng.next_range(-15.0, 15.0),
                            vy: self.rng.next_range(-12.0, -1.0),
                            life: self.rng.next_range(0.5, 1.2),
                            color: (50, 50, 55),
                            is_background: false,
                        });
                        self.splashes.push(Splash {
                            x: self.bird_x,
                            y: self.bird_y,
                            vx: self.rng.next_range(-18.0, 18.0),
                            vy: self.rng.next_range(-14.0, 2.0),
                            life: self.rng.next_range(0.4, 0.9),
                            color: (255, 235, 140),
                            is_background: false,
                        });
                    }
                }
            }
            BirdState::Dead => {
                self.bird_timer -= delta;
                if self.bird_timer <= 0.0 {
                    if !self.perch_points.is_empty() {
                        let p_idx = self.rng.next_usize(self.perch_points.len());
                        self.bird_perch_x = self.perch_points[p_idx].0 as f32;
                        self.bird_perch_y = self.perch_points[p_idx].1 as f32;
                    }
                    self.bird_state = BirdState::Sitting;
                    self.bird_timer = self.rng.next_range(5.0, 15.0);
                    self.bird_x = self.bird_perch_x;
                    self.bird_y = self.bird_perch_y;
                }
            }
        }
    }
}
