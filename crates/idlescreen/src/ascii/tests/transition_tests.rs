// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::draw::{
    apply_anamorphic_lens_flare, banner_bevel_color, neon_overdrive_rgb, paint_transition,
    theme_bevel_band,
};
use super::super::sys_info::{
    GlyphFontStyle, SystemTheme, ThemeColorScheme, detect_system_theme, parse_color_str,
};
use crate::cell::TerminalCell;
use crate::wayland::blit::{convert_argb8888_to_2101010_in_place, pack_argb2101010};
use crate::wayland::glyph_font::get_glyph_bitmap_for_style;
use crate::wayland::presenter::{WaylandState, viewport_buffer_dimensions};
use wayland_client::protocol::wl_shm;

#[test]
fn test_paint_transition_bounds() {
    let mut from = vec![TerminalCell::default(); 80 * 24];
    let mut to = vec![TerminalCell::default(); 80 * 24];
    let mut out = vec![TerminalCell::default(); 80 * 24];

    from[0].ch = 'A';
    to[0].ch = 'B';

    paint_transition(&from, &to, &mut out, 0.0, 80, 24);
    assert_eq!(out[0].ch, 'A');

    paint_transition(&from, &to, &mut out, 1.0, 80, 24);
    assert_eq!(out[0].ch, 'B');

    // Partial transition
    paint_transition(&from, &to, &mut out, 0.5, 80, 24);
    assert!(out[0].ch == 'A' || out[0].ch == 'B');
}

#[test]
#[allow(unsafe_code)]
fn test_system_theme_adaptation_and_parsing() {
    assert_eq!(parse_color_str("#ff4080"), Some((255, 64, 128)));
    assert_eq!(parse_color_str("0x10a0ff"), Some((16, 160, 255)));
    assert_eq!(parse_color_str("81, 162, 218"), Some((81, 162, 218)));
    assert_eq!(parse_color_str(""), None);

    assert_eq!(
        GlyphFontStyle::parse("JetBrains Mono"),
        GlyphFontStyle::CyberSharp
    );
    assert_eq!(
        GlyphFontStyle::parse("Ubuntu Mono"),
        GlyphFontStyle::RetroRound
    );
    assert_eq!(
        GlyphFontStyle::parse("Default"),
        GlyphFontStyle::ArcadeBlock
    );

    let light = SystemTheme::from_parts(
        (255, 100, 50),
        ThemeColorScheme::Light,
        GlyphFontStyle::CyberSharp,
    );
    assert_eq!(light.bg_rgb, (10, 18, 34));

    let dark = SystemTheme::from_parts(
        (81, 162, 218),
        ThemeColorScheme::Dark,
        GlyphFontStyle::ArcadeBlock,
    );
    assert_eq!(dark.bg_rgb, (0, 0, 0));

    let detected = crate::ascii::sys_info::resolve_system_theme_with(dark, |k| match k {
        "IDLE_THEME_ACCENT" => Some("#ff2090".to_string()),
        "IDLE_THEME_MODE" => Some("light".to_string()),
        "IDLE_FONT_STYLE" => Some("cyber-sharp".to_string()),
        _ => None,
    });
    assert_eq!(detected.accent_rgb, (255, 32, 144));
    assert_eq!(detected.scheme, ThemeColorScheme::Light);
    assert_eq!(detected.font_style, GlyphFontStyle::CyberSharp);
    assert_eq!(detected.bg_rgb, (10, 18, 34));

    let live = detect_system_theme();
    assert!(live.bg_rgb == (0, 0, 0) || live.bg_rgb == (10, 18, 34));

    let bevel_top = banner_bevel_color(0, 0, 10, false);
    let bevel_bot = banner_bevel_color(10, 0, 10, false);
    assert_ne!(bevel_top, bevel_bot);
    assert_eq!(theme_bevel_band(6, (255, 32, 144)), (255, 32, 144));
}

#[test]
fn test_anamorphic_lens_flare_and_neon_overdrive() {
    let boosted = neon_overdrive_rgb((40, 120, 80));
    assert!(boosted.1 >= 175);
    assert!(boosted.1 > boosted.0);

    let (cols, rows) = (30, 8);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    let y = 3;
    let hot_x = 12;
    let gleam_pos = hot_x + y * 2;
    grid[y * cols + hot_x] = TerminalCell {
        ch: '█',
        fg: (245, 255, 255),
        bg: (0, 0, 0),
        bold: true,
    };

    apply_anamorphic_lens_flare(&mut grid, cols, rows, gleam_pos, (0, 0, 0));
    // Horizontal anamorphic streak wings should populate adjacent empty cells
    assert_eq!(grid[y * cols + (hot_x - 1)].ch, '▓');
    assert_eq!(grid[y * cols + (hot_x + 1)].ch, '▓');
    assert_eq!(grid[y * cols + (hot_x + 3)].ch, '▒');
    assert_eq!(grid[y * cols + (hot_x + 6)].ch, '░');
    // Vertical soft halo should populate y-1 and y+1
    assert_eq!(grid[(y - 1) * cols + hot_x].ch, '░');
    assert_eq!(grid[(y + 1) * cols + hot_x].ch, '░');
}

#[test]
fn test_hdr_10bit_packing_4k_viewport_and_glyph_styles() {
    let white_10 = pack_argb2101010((255, 255, 255));
    assert_eq!(white_10, 0xFFFF_FFFF);
    let black_10 = pack_argb2101010((0, 0, 0));
    assert_eq!(black_10, 0xC000_0000);

    let mut buf = [0xFF80_4020u32, 0xFFFF_FFFFu32];
    convert_argb8888_to_2101010_in_place(&mut buf);
    assert_eq!(buf[1], 0xFFFF_FFFF);
    assert_eq!(buf[0] >> 30, 0b11);

    let mut state = WaylandState::new();
    assert_eq!(
        state.preferred_shm_format(),
        (wl_shm::Format::Argb8888, false)
    );
    state.shm_formats.push(wl_shm::Format::Xrgb2101010);
    assert_eq!(
        state.preferred_shm_format(),
        (wl_shm::Format::Xrgb2101010, true)
    );
    state.shm_formats.push(wl_shm::Format::Argb2101010);
    assert_eq!(
        state.preferred_shm_format(),
        (wl_shm::Format::Argb2101010, true)
    );

    // 1080p uses 1x native grid (960x576), 4K (3840x2160) uses 2x Hi-Res grid (1920x1152)
    assert_eq!(viewport_buffer_dimensions(1920, 1080, 120, 36), (960, 576));
    assert_eq!(
        viewport_buffer_dimensions(3840, 2160, 120, 36),
        (1920, 1152)
    );

    let b_block = get_glyph_bitmap_for_style('█', GlyphFontStyle::ArcadeBlock);
    let b_sharp = get_glyph_bitmap_for_style('█', GlyphFontStyle::CyberSharp);
    let b_round = get_glyph_bitmap_for_style('█', GlyphFontStyle::RetroRound);
    assert_ne!(b_block, b_sharp);
    assert_ne!(b_block, b_round);
}
