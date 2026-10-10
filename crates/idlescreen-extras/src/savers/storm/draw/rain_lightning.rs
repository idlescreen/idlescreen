use crate::runner::TerminalCell;
use crate::storm::Storm;

impl Storm {
    pub(crate) fn draw_rain(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
        rain_char: char,
        is_background: bool,
    ) {
        let fade = self.intro_fade.clamp(0.0, 1.0);
        for drop in &self.drops {
            if drop.is_background == is_background {
                let cx = drop.x as usize;
                let cy = drop.y as usize;
                if cx < cols && cy < rows {
                    let bold = drop.layer == 2;
                    let fg = (
                        (drop.color.0 as f32 * fade) as u8,
                        (drop.color.1 as f32 * fade) as u8,
                        (drop.color.2 as f32 * fade) as u8,
                    );
                    grid[cy * cols + cx] = TerminalCell {
                        ch: rain_char,
                        fg,
                        bg: bg_color,
                        bold,
                    };
                }
            }
        }
    }

    pub(crate) fn draw_splashes(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
        is_background: bool,
    ) {
        let fade = self.intro_fade.clamp(0.0, 1.0);
        for s in &self.splashes {
            if s.is_background == is_background {
                let cx = s.x as usize;
                let cy = s.y as usize;
                if cx < cols && cy < rows {
                    let life_factor = (s.life * 2.0).min(1.0) * fade;
                    let fg = (
                        (s.color.0 as f32 * life_factor) as u8,
                        (s.color.1 as f32 * life_factor) as u8,
                        (s.color.2 as f32 * life_factor) as u8,
                    );
                    let ch = if s.vy < 0.0 {
                        'o'
                    } else if s.life > 0.35 {
                        '*'
                    } else if s.life > 0.18 {
                        '+'
                    } else {
                        '.'
                    };
                    grid[cy * cols + cx] = TerminalCell {
                        ch,
                        fg,
                        bg: bg_color,
                        bold: false,
                    };
                }
            }
        }
    }

    pub(crate) fn draw_lightning_bolts(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        bg_color: (u8, u8, u8),
        in_flash: bool,
        is_bg: bool,
    ) {
        if in_flash && self.lightning_is_background == is_bg {
            let fade = self.intro_fade.clamp(0.0, 1.0);
            let flash_t = (self.lightning_flash / 0.32).clamp(0.0, 1.0);
            let accent = self.cached_accent;
            // Soft accent-tinted bolt at peak, cooler white on the tail
            let base = if is_bg {
                (210.0, 215.0, 240.0)
            } else {
                (255.0, 255.0, 255.0)
            };
            let mix = 0.22 * flash_t;
            let bolt_fg = (
                ((base.0 * (1.0 - mix) + accent.0 as f32 * mix) * flash_t * fade) as u8,
                ((base.1 * (1.0 - mix) + accent.1 as f32 * mix) * flash_t * fade) as u8,
                ((base.2 * (1.0 - mix) + accent.2 as f32 * mix) * flash_t * fade) as u8,
            );
            for bolt in &self.lightning_bolts {
                for i in 0..bolt.len() {
                    let (lx, ly) = bolt[i];
                    if lx < cols && ly < rows {
                        let ch = if i == 0 {
                            if bolt.len() > 1 {
                                let (nx, _) = bolt[1];
                                if nx > lx {
                                    '\\'
                                } else if nx < lx {
                                    '/'
                                } else {
                                    '|'
                                }
                            } else {
                                '|'
                            }
                        } else {
                            let (px, _) = bolt[i - 1];
                            if px < lx {
                                '\\'
                            } else if px > lx {
                                '/'
                            } else {
                                '|'
                            }
                        };
                        grid[ly * cols + lx] = TerminalCell {
                            ch,
                            fg: bolt_fg,
                            bg: bg_color,
                            bold: flash_t > 0.35,
                        };
                    }
                }
            }
        }
    }
}
