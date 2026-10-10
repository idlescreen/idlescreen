use super::spike::draw_spike;
use crate::chaos::Chaos;
use crate::runner::TerminalCell;

impl Chaos {
    pub(crate) fn draw_stars(
        &self,
        grid: &mut [TerminalCell],
        cols: usize,
        rows: usize,
        accent: (u8, u8, u8),
    ) {
        // Top-4 flare selection by excitation — fixed-size insertion, no
        // per-frame Vec allocs (was: collect + sort + take + collect).
        const MAX_FLARES: usize = 4;
        let mut flare_idx = [0usize; MAX_FLARES];
        let mut flare_score = [f32::MIN; MAX_FLARES];
        let mut n_flares = 0usize;
        for (i, star) in self.stars.iter().enumerate() {
            if star.excitation <= 0.8 {
                continue;
            }
            if n_flares == MAX_FLARES && star.excitation <= flare_score[MAX_FLARES - 1] {
                continue;
            }
            let mut pos = n_flares.min(MAX_FLARES - 1);
            while pos > 0 && star.excitation > flare_score[pos - 1] {
                flare_score[pos] = flare_score[pos - 1];
                flare_idx[pos] = flare_idx[pos - 1];
                pos -= 1;
            }
            flare_score[pos] = star.excitation;
            flare_idx[pos] = i;
            n_flares = (n_flares + 1).min(MAX_FLARES);
        }
        let allowed_flares = &flare_idx[..n_flares];

        for (i, star) in self.stars.iter().enumerate() {
            let sx = (star.x * cols as f32) as usize;
            let sy = (star.y * rows as f32) as usize;

            if sx < cols && sy < rows {
                let sparkle = f32::midpoint((self.time_elapsed * 2.5 + star.phase).sin(), 1.0);
                let brightness = (sparkle * 0.35 + star.excitation * 0.65).min(1.0);

                let (r, g, b) = if star.excitation > 0.05 {
                    let blend = star.excitation.min(1.0);
                    (
                        (160.0 * (1.0 - blend) + accent.0 as f32 * blend).min(255.0) as u8,
                        (180.0 * (1.0 - blend) + accent.1 as f32 * blend).min(255.0) as u8,
                        (220.0 * (1.0 - blend) + accent.2 as f32 * blend).min(255.0) as u8,
                    )
                } else {
                    (
                        (110.0 + brightness * 70.0) as u8,
                        (120.0 + brightness * 75.0) as u8,
                        (140.0 + brightness * 80.0) as u8,
                    )
                };

                let ch = if star.excitation > 0.8 {
                    '✹'
                } else if star.excitation > 0.4 {
                    '✦'
                } else {
                    star.ch
                };

                grid[sy * cols + sx] = TerminalCell {
                    ch,
                    fg: (r, g, b),
                    bg: grid[sy * cols + sx].bg,
                    bold: sparkle > 0.6 || star.excitation > 0.3,
                };

                let is_excited = allowed_flares.contains(&i);
                if is_excited {
                    let flare_intensity = ((star.excitation - 0.8) / 0.7 + 0.5).min(1.5);

                    let h_len = 12;
                    for dx in 1..h_len {
                        let alpha = (120.0 * flare_intensity).max(30.0) as u8;
                        let fade = alpha.saturating_sub((dx * (110 / h_len)) as u8);
                        if fade > 10 {
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32 + dx,
                                sy as i32,
                                '─',
                                fade,
                                0.75,
                                45,
                                accent,
                            );
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32 - dx,
                                sy as i32,
                                '─',
                                fade,
                                0.75,
                                45,
                                accent,
                            );
                        }
                    }

                    let v_len = 5;
                    for dy in 1..v_len {
                        let alpha = (90.0 * flare_intensity).max(20.0) as u8;
                        let fade = alpha.saturating_sub((dy * (80 / v_len)) as u8);
                        if fade > 10 {
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32,
                                sy as i32 + dy,
                                '│',
                                fade,
                                0.75,
                                30,
                                accent,
                            );
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32,
                                sy as i32 - dy,
                                '│',
                                fade,
                                0.75,
                                30,
                                accent,
                            );
                        }
                    }

                    let d_len = 3;
                    for d in 1..=d_len {
                        let alpha = (70.0 * flare_intensity).max(15.0) as u8;
                        let fade = alpha.saturating_sub((d * (60 / d_len)) as u8);
                        if fade > 10 {
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32 + d,
                                sy as i32 - d,
                                '/',
                                fade,
                                0.65,
                                20,
                                accent,
                            );
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32 - d,
                                sy as i32 + d,
                                '/',
                                fade,
                                0.65,
                                20,
                                accent,
                            );
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32 - d,
                                sy as i32 - d,
                                '\\',
                                fade,
                                0.65,
                                20,
                                accent,
                            );
                            draw_spike(
                                grid,
                                cols,
                                rows,
                                sx as i32 + d,
                                sy as i32 + d,
                                '\\',
                                fade,
                                0.65,
                                20,
                                accent,
                            );
                        }
                    }
                }
            }
        }
    }
}
