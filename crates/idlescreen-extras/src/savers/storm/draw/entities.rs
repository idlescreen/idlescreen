use crate::runner::TerminalCell;
use crate::storm::Storm;
use crate::storm::types::{AnimalType, BirdState};

impl Storm {
    pub(crate) fn draw_midground_animals(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        if let Some(ref animal) = self.active_animal
            && animal.animal_type == AnimalType::Bigfoot
        {
            let ax = animal.x.round() as i32;
            let ay = animal.y.round() as i32;
            if ax >= 0 && ax < cols as i32 {
                let ux = ax as usize;
                // Head: '▲'
                if ay >= 0 && ay < rows as i32 {
                    grid[(ay as usize) * cols + ux] = TerminalCell {
                        ch: '▲',
                        fg: (55, 45, 40),
                        bg: bg_color,
                        bold: false,
                    };
                }
                // Body: '█'
                if ay + 1 >= 0 && ay + 1 < rows as i32 {
                    grid[((ay + 1) as usize) * cols + ux] = TerminalCell {
                        ch: '█',
                        fg: (55, 45, 40),
                        bg: bg_color,
                        bold: false,
                    };
                }
                // Legs: '╩' or '║'
                if ay + 2 >= 0 && ay + 2 < rows as i32 {
                    let ch = if animal.frame_toggle { '╩' } else { '║' };
                    grid[((ay + 2) as usize) * cols + ux] = TerminalCell {
                        ch,
                        fg: (55, 45, 40),
                        bg: bg_color,
                        bold: false,
                    };
                }
            }
        }
    }

    pub(crate) fn draw_logo_cells(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
        cold_accent: (u8, u8, u8),
    ) {
        for cell in &self.logo_cells {
            if cell.active {
                if cell.x >= cols || cell.y >= rows {
                    continue;
                }
                let glow_val = cell.glow.min(1.0);
                let fg_r =
                    (cold_accent.0 as f32 + (255.0 - cold_accent.0 as f32) * glow_val * 0.7) as u8;
                let fg_g =
                    (cold_accent.1 as f32 + (255.0 - cold_accent.1 as f32) * glow_val * 0.7) as u8;
                let fg_b =
                    (cold_accent.2 as f32 + (255.0 - cold_accent.2 as f32) * glow_val * 0.7) as u8;
                grid[cell.y * cols + cell.x] = TerminalCell {
                    ch: cell.ch,
                    fg: (fg_r, fg_g, fg_b),
                    bg: bg_color,
                    bold: true,
                };
            }
        }

        for cell in &self.logo_cells {
            if cell.active && cell.water > 0.15 && cell.y > 0 {
                let above_x = cell.x;
                let above_y = cell.y - 1;
                if above_x >= cols || above_y >= rows {
                    continue;
                }
                let has_above_active = self
                    .logo_cells
                    .iter()
                    .any(|c| c.active && c.x == above_x && c.y == above_y);
                if !has_above_active {
                    let w_level = cell.water;
                    let ch = if w_level > 1.8 {
                        '~'
                    } else if w_level > 0.9 {
                        '_'
                    } else {
                        '.'
                    };

                    grid[above_y * cols + above_x] = TerminalCell {
                        ch,
                        fg: (100, 135, 170),
                        bg: bg_color,
                        bold: w_level > 1.0,
                    };
                }
            }
        }
    }

    pub(crate) fn draw_foreground_animals(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        if let Some(ref animal) = self.active_animal {
            let ax = animal.x.round() as i32;
            let ay = animal.y.round() as i32;
            if ax >= 0 && ax < cols as i32 {
                let ux = ax as usize;
                match animal.animal_type {
                    AnimalType::Bear => {
                        if ay >= 0 && ay < rows as i32 {
                            grid[(ay as usize) * cols + ux] = TerminalCell {
                                ch: '∩',
                                fg: (80, 55, 35),
                                bg: bg_color,
                                bold: false,
                            };
                        }
                        if ay + 1 >= 0 && ay + 1 < rows as i32 {
                            grid[((ay + 1) as usize) * cols + ux] = TerminalCell {
                                ch: '█',
                                fg: (80, 55, 35),
                                bg: bg_color,
                                bold: false,
                            };
                        }
                    }
                    AnimalType::Deer => {
                        if ay >= 0 && ay < rows as i32 {
                            grid[(ay as usize) * cols + ux] = TerminalCell {
                                ch: '¥',
                                fg: (160, 110, 65),
                                bg: bg_color,
                                bold: false,
                            };
                        }
                        if ay + 1 >= 0 && ay + 1 < rows as i32 {
                            let ch = if animal.frame_toggle { '╩' } else { '╚' };
                            grid[((ay + 1) as usize) * cols + ux] = TerminalCell {
                                ch,
                                fg: (160, 110, 65),
                                bg: bg_color,
                                bold: false,
                            };
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn draw_bird(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
    ) {
        if self.bird_state != BirdState::Dead {
            let bx = self.bird_x.round() as i32;
            let by = self.bird_y.round() as i32;
            if bx >= 0 && bx < cols as i32 && by >= 0 && by < rows as i32 {
                let ubx = bx as usize;
                let uby = by as usize;
                let ch = match self.bird_state {
                    BirdState::Sitting => 'v',
                    BirdState::Scared => 'V',
                    BirdState::Flying => {
                        if self.bird_wing_flap {
                            'w'
                        } else {
                            'v'
                        }
                    }
                    BirdState::Explores => {
                        if self.bird_wing_flap {
                            'W'
                        } else {
                            'V'
                        }
                    }
                    _ => 'v',
                };

                let fg = match self.bird_state {
                    BirdState::Scared => (255, 100, 100),
                    BirdState::Explores => {
                        let flash_cycle = (self.phase_timer * 15.0) as usize % 3;
                        match flash_cycle {
                            0 => (255, 255, 255),
                            1 => (255, 235, 140),
                            _ => (100, 220, 255),
                        }
                    }
                    _ => (200, 210, 230),
                };

                grid[uby * cols + ubx] = TerminalCell {
                    ch,
                    fg,
                    bg: bg_color,
                    bold: self.bird_state == BirdState::Scared
                        || self.bird_state == BirdState::Explores,
                };

                if self.bird_state == BirdState::Scared && uby > 0 {
                    let alert_str = "Chirp!";
                    for (i, ach) in alert_str.chars().enumerate() {
                        let ax = ubx + i;
                        if ax < cols {
                            grid[(uby - 1) * cols + ax] = TerminalCell {
                                ch: ach,
                                fg: (255, 200, 100),
                                bg: bg_color,
                                bold: true,
                            };
                        }
                    }
                }
            }
        }
    }
}
