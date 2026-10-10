use crate::storm::Storm;
use crate::storm::types::{Animal, AnimalState, AnimalType};

impl Storm {
    pub fn update_scenery_and_animals(&mut self, delta: f32, cols: usize, rows: usize) {
        self.animal_spawn_timer -= delta;
        if self.animal_spawn_timer <= 0.0 && self.active_animal.is_none() {
            // Longer, staggered gaps so wildlife never stacks or feels clockwork
            self.animal_spawn_timer = self.rng.next_range(38.0, 85.0);
            let roll = self.rng.next_range(0.0, 1.0);
            let animal_type = if roll < 0.50 {
                AnimalType::Deer
            } else if roll < 0.85 {
                AnimalType::Bear
            } else {
                AnimalType::Bigfoot
            };

            let spawn_left = self.rng.next_bool(0.5);
            let base_speed = match animal_type {
                AnimalType::Deer => 4.5f32,
                AnimalType::Bear => 1.8f32,
                AnimalType::Bigfoot => 1.2f32,
            };

            let ay = if animal_type == AnimalType::Bigfoot {
                rows.saturating_sub(4) as f32
            } else {
                rows.saturating_sub(3) as f32
            };

            self.active_animal = Some(Animal {
                x: if spawn_left { -3.0 } else { cols as f32 + 3.0 },
                y: ay,
                vx: if spawn_left { base_speed } else { -base_speed },
                animal_type,
                state: AnimalState::Walking,
                timer: self.rng.next_range(5.0, 9.0),
                frame_toggle: false,
            });

            self.subtitle = match animal_type {
                AnimalType::Deer => "[A deer wanders out of the forest]".to_string(),
                AnimalType::Bear => "[A heavy brown bear walks out of the woods]".to_string(),
                AnimalType::Bigfoot => {
                    "[Unidentified creature rustling in the midground trees...]".to_string()
                }
            };
            self.subtitle_timer = 3.0;
        }

        if let Some(ref mut animal) = self.active_animal {
            animal.timer -= delta;

            if self.rng.next_bool(0.08) {
                animal.frame_toggle = !animal.frame_toggle;
            }

            match animal.state {
                AnimalState::Walking => {
                    animal.x += animal.vx * delta;
                    if animal.timer <= 0.0 {
                        if self.rng.next_bool(0.40) && animal.animal_type != AnimalType::Bigfoot {
                            animal.state = AnimalState::Idle;
                            animal.timer = self.rng.next_range(3.0, 6.0);
                            self.subtitle = match animal.animal_type {
                                AnimalType::Deer => "[Deer grazing on mossy ground]".to_string(),
                                AnimalType::Bear => "[Bear sitting down to rest]".to_string(),
                                _ => String::new(),
                            };
                            self.subtitle_timer = 2.0;
                        } else {
                            animal.timer = self.rng.next_range(4.0, 8.0);
                        }
                    }
                }
                AnimalState::Idle => {
                    if animal.timer <= 0.0 {
                        animal.state = AnimalState::Walking;
                        animal.timer = self.rng.next_range(4.0, 8.0);
                        self.subtitle = match animal.animal_type {
                            AnimalType::Deer => "[Deer walks on]".to_string(),
                            AnimalType::Bear => "[Bear lumbering forward]".to_string(),
                            _ => String::new(),
                        };
                        self.subtitle_timer = 1.8;
                    }
                }
                AnimalState::Startled => {
                    if animal.timer <= 0.0 {
                        animal.state = AnimalState::WalkingOff;
                        let run_speed = match animal.animal_type {
                            AnimalType::Deer => 11.0f32,
                            AnimalType::Bear => 6.0f32,
                            _ => 3.0f32,
                        };
                        animal.vx = animal.vx.signum() * run_speed;
                    }
                }
                AnimalState::WalkingOff => {
                    animal.x += animal.vx * delta;
                }
            }
        }

        if let Some(ref animal) = self.active_animal
            && (animal.x < -6.0 || animal.x > cols as f32 + 6.0)
        {
            self.active_animal = None;
        }

        if self.subtitle_timer > 0.0 {
            self.subtitle_timer -= delta;
            if self.subtitle_timer <= 0.0 {
                self.subtitle.clear();
            }
        }

        if self.wind.abs() > 8.5 && self.rng.next_bool(0.01) && self.subtitle_timer <= 0.0 {
            self.subtitle = "Warning: Severe gale force wind gusts".to_string();
            self.subtitle_timer = 2.5;
        }
    }
}
