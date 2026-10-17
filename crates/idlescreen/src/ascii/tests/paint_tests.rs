// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::draw::{is_pixel_art_char, place_banner_centered};
use super::super::effects::EffectKind;
use crate::cell::TerminalCell;
use crate::wayland::blit::{GLYPH_HEIGHT, GLYPH_WIDTH, blit_cells_to_argb};
use crate::wayland::glyph_font::{draw_preview_hud, get_glyph_bitmap};

#[test]
fn test_place_banner_centered() {
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    let lines = vec![
        "█████".to_string(),
        "█   █".to_string(),
        "█████".to_string(),
    ];
    let (x, y, w, h) = place_banner_centered(&mut grid, 80, 24, &lines, (255, 255, 255), (0, 0, 0));
    assert_eq!(w, 5);
    assert_eq!(h, 3);
    assert_eq!(x, (80 - 5) / 2);
    assert_eq!(y, (24 - 3) / 2);

    let idx = y * 80 + x;
    assert_eq!(grid[idx].ch, '█');
}

#[test]
fn new_13_scenes_complete_in_64_frames_with_block_elements() {
    let art_data = "█████\n█   █\n█████";
    assert_eq!(EffectKind::NEW_SCENES.len(), 13);
    for &kind in &EffectKind::NEW_SCENES {
        let mut effect = crate::ascii::scenes::create_scene(kind);
        let config = ttfx::engine::terminal::TerminalConfig {
            canvas_width: 80,
            canvas_height: 24,
            ignore_terminal_dimensions: true,
            anchor_text: ttfx::engine::canvas::Anchor::C,
            frame_rate: 60,
            ..Default::default()
        };
        let clock = ttfx::engine::ctx::Clock::virtual_with_frame_rate(60);
        let rng = ttfx::utils::rng::Rng::seeded(42);
        let mut ctx = ttfx::engine::ctx::EngineCtx::new(art_data, config, rng, clock).unwrap();
        effect.build(&mut ctx).unwrap();

        let mut frame_count = 0usize;
        while let Some(frame) = effect.next_frame(&mut ctx) {
            frame_count += 1;
            for line in frame.lines() {
                for ch in line.chars() {
                    if ch == '\x1b' {
                        break;
                    }
                    assert!(
                        is_pixel_art_char(ch),
                        "New scene {} emitted non-block char {:?}",
                        kind.name(),
                        ch
                    );
                }
            }
        }
        assert_eq!(
            frame_count,
            64,
            "New scene {} must complete entry animation in 64 frames",
            kind.name()
        );
    }
}

#[test]
fn test_draw_preview_hud_and_glyph_blitting() {
    let (cols, rows) = (80, 24);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    draw_preview_hud(&mut grid, cols, rows, EffectKind::Plasma, false);
    let bottom_row: String = grid[(rows - 1) * cols..]
        .iter()
        .map(|cell| cell.ch)
        .collect();
    assert!(
        bottom_row.contains("SCENE 28/52: PLASMA"),
        "bottom row was: {bottom_row:?}"
    );
    assert!(bottom_row.contains("[←/→: STEP | Q/ESC: EXIT]"));

    draw_preview_hud(&mut grid, cols, rows, EffectKind::Aurora, true);
    let tour_row: String = grid[(rows - 1) * cols..]
        .iter()
        .map(|cell| cell.ch)
        .collect();
    assert!(
        tour_row.contains("TOUR 01/52: AURORA"),
        "tour row was: {tour_row:?}"
    );

    draw_preview_hud(&mut grid, cols, rows, EffectKind::Zero, true);
    let tour_end_row: String = grid[(rows - 1) * cols..]
        .iter()
        .map(|cell| cell.ch)
        .collect();
    assert!(
        tour_end_row.contains("TOUR 52/52: ZERO"),
        "tour end row was: {tour_end_row:?}"
    );

    // Verify 8x16 glyph bitmaps for HUD characters have non-empty pixels and blit cleanly
    for ch in ['A', 'P', '0', '9', '/', '[', ']', '←', '→'] {
        let bmp = get_glyph_bitmap(ch);
        assert!(
            bmp.iter().any(|&row| row != 0),
            "glyph {ch:?} should have non-zero bitmap bits"
        );
    }
    let pixel_w = cols * GLYPH_WIDTH;
    let pixel_h = rows * GLYPH_HEIGHT;
    let mut buffer = vec![0u32; pixel_w * pixel_h];
    blit_cells_to_argb(&grid, cols, rows, &mut buffer, pixel_w, pixel_h);
    let bottom_pixels = &buffer[(rows - 1) * GLYPH_HEIGHT * pixel_w..];
    assert!(bottom_pixels.iter().any(|&px| px != 0xFF00_0000));
}

#[test]
fn test_starfield_backdrop_variation_and_randomization() {
    use super::super::Ascii;
    use super::super::draw::paint_starfield_backdrop;

    let (cols, rows) = (80, 24);
    let mut distinct_palettes = std::collections::HashSet::new();
    let mut distinct_layouts = std::collections::HashSet::new();

    for variant in 0..8u64 {
        let seed = ((variant + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) & !0x3F)
            | (((variant * 3) & 7) << 3)
            | (variant & 7);
        let mut grid = vec![TerminalCell::default(); cols * rows];
        paint_starfield_backdrop(&mut grid, cols, rows, 24, seed, (0, 0, 0));

        assert!(grid.iter().all(|c| is_pixel_art_char(c.ch)));
        let non_empty: Vec<(usize, char, (u8, u8, u8))> = grid
            .iter()
            .enumerate()
            .filter(|(_, c)| c.ch != ' ')
            .map(|(idx, c)| (idx, c.ch, c.fg))
            .collect();
        assert!(
            !non_empty.is_empty(),
            "starfield variant {variant} must emit stars"
        );
        for &(_, _, fg) in &non_empty {
            distinct_palettes.insert(fg);
        }
        distinct_layouts.insert(non_empty.iter().map(|&(idx, _, _)| idx).collect::<Vec<_>>());
    }

    assert!(
        distinct_palettes.len() >= 16,
        "expected rich multi-theme starfield colors across 8 themes, got {}",
        distinct_palettes.len()
    );
    assert_eq!(
        distinct_layouts.len(),
        8,
        "each scene seed must produce distinct randomized star positions"
    );

    // Stepping or cycling scenes in Ascii must advance backdrop_seed and vary starfield themes
    let mut saver = Ascii::new_superfedora();
    saver.init(cols, rows);
    let seed_a = saver.backdrop_seed;
    saver.step_effect(&EffectKind::ALL, 1);
    let seed_b = saver.backdrop_seed;
    assert_ne!(seed_a, seed_b);
    assert_ne!(
        seed_a & 7,
        seed_b & 7,
        "consecutive scenes must switch starfield theme"
    );
    assert_ne!(
        (seed_a >> 3) & 7,
        (seed_b >> 3) & 7,
        "consecutive scenes must switch starfield drift vector"
    );
}

#[test]
fn mmx_snes_halfblock_sprites_destroy_superfedora_and_teleport_out() {
    use super::super::Ascii;
    use std::time::Duration;

    let (cols, rows) = (120, 36);
    let mut saver = Ascii::new_superfedora().with_dwell(0.5);
    saver.init(cols, rows);
    saver.pin_effect(EffectKind::Mmx);
    let mut grid = vec![TerminalCell::default(); cols * rows];

    let (mut saw_gem, mut saw_face, mut saw_half, mut saw_crush, mut saw_flash) =
        (false, false, false, false, false);
    let (mut banner_start, mut banner_after) = (0usize, 0usize);
    let initial_blocks = saver.frame_grid.iter().filter(|c| c.ch == '█').count();

    for step_idx in 1..=84 {
        saver.update(Duration::from_millis(16), cols, rows);
        if step_idx <= 35 {
            for (idx, cell) in saver.frame_grid.iter().enumerate() {
                if matches!(cell.ch, '▀' | '▄') {
                    let col = idx % cols;
                    assert!(
                        col < 15,
                        "MMX sprite at col {col} overlapped SUPERFEDORA (t_left=15) on step {step_idx}"
                    );
                }
            }
            let cur_blocks = saver.frame_grid.iter().filter(|c| c.ch == '█').count();
            assert_eq!(
                cur_blocks, initial_blocks,
                "MMX overlapped SUPERFEDORA blocks on step {step_idx}"
            );
        }
        saver.draw(&mut grid, cols, rows);
        for c in &grid {
            saw_gem |= c.fg == (240, 64, 16) || c.bg == (240, 64, 16);
            saw_face |= c.fg == (248, 176, 128) || c.bg == (248, 176, 128);
            saw_crush |= c.fg == (248, 80, 168) || c.bg == (248, 80, 168);
            saw_flash |= c.ch == '█' && c.fg == (255, 255, 255);
            saw_half |= c.ch == '▀' && c.bg != (0, 0, 0);
        }
        if step_idx == 14 {
            banner_start = grid.iter().filter(|c| c.ch == '█').count();
        } else if step_idx == 68 {
            banner_after = grid.iter().filter(|c| c.ch == '█').count();
        }
    }

    assert!(saw_gem && saw_face && saw_half && saw_crush && saw_flash);
    assert!(banner_start > 200, "got {banner_start} blocks");
    assert_eq!(banner_after, 0);
    assert!(saver.is_settled());

    for &(c, r) in &[(80, 18), (44, 16), (40, 8), (10, 5)] {
        saver.init(c, r);
        saver.pin_effect(EffectKind::Mmx);
        let mut small = vec![TerminalCell::default(); c * r];
        for _ in 0..40 {
            saver.update(Duration::from_millis(16), c, r);
            saver.draw(&mut small, c, r);
        }
    }
}
