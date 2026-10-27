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
// The 10-bit packing helpers are only used by the HDR test, which needs
// the Wayland presenter.
#[cfg(feature = "product")]
use crate::raster::blit::{convert_argb8888_to_2101010_in_place, pack_argb2101010};
#[cfg(feature = "product")]
use crate::raster::glyph_font::get_glyph_bitmap_for_style;
// The presenter and the Wayland protocol bindings are product-only.
#[cfg(feature = "product")]
use crate::wayland::presenter::{WaylandState, viewport_buffer_dimensions};
#[cfg(feature = "product")]
use wayland_client::protocol::wl_shm;

#[test]
fn test_paint_transition_bounds() {
    let mut from = vec![TerminalCell::default(); 80 * 24];
    let mut to = vec![TerminalCell::default(); 80 * 24];
    let mut out = vec![TerminalCell::default(); 80 * 24];

    from[0].ch = 'A';
    to[0].ch = 'B';

    paint_transition(&from, &to, &mut out, 0.0, 80, 24, 7);
    assert_eq!(out[0].ch, 'A');

    paint_transition(&from, &to, &mut out, 1.0, 80, 24, 7);
    assert_eq!(out[0].ch, 'B');

    // Partial transition
    paint_transition(&from, &to, &mut out, 0.5, 80, 24, 7);
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

    use crate::ascii::sys_info::theme_probe::{
        extract_conf_color, parse_cosmic_ron_rgb, probe_config_files,
    };
    assert_eq!(
        parse_cosmic_ron_rgb("(red: 1.0, green: 0.5, blue: 0.0)"),
        Some((255, 128, 0))
    );
    assert_eq!(
        parse_cosmic_ron_rgb("(red: -0.5, green: 0.5, blue: 0.5)"),
        None
    );
    assert_eq!(
        parse_cosmic_ron_rgb("(red: 999.0, green: 0.5, blue: 0.5)"),
        None
    );
    assert_eq!(
        extract_conf_color(
            "inactive_border = rgba(333333ff)\ncol.active_border = rgb(ff2090) rgb(00ff99) 45deg"
        ),
        Some((255, 32, 144))
    );

    let tmp = std::env::temp_dir().join(format!("idlescreen-theme-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(tmp.join("cosmic/com.system76.CosmicTheme.Mode/v1"));
    let _ = std::fs::create_dir_all(tmp.join("cosmic/com.system76.CosmicTheme.Light/v1"));
    let _ = std::fs::create_dir_all(tmp.join("gtk-4.0"));
    let _ = std::fs::write(
        tmp.join("cosmic/com.system76.CosmicTheme.Mode/v1/is_dark"),
        "false\n",
    );
    let _ = std::fs::write(
        tmp.join("cosmic/com.system76.CosmicTheme.Light/v1/accent"),
        "(red: 0.2, green: 0.8, blue: 1.0)",
    );
    let _ = std::fs::write(
        tmp.join("gtk-4.0/settings.ini"),
        "[Settings]\ngtk-font-name=JetBrains Mono 11\n",
    );
    let (mut acc, mut sch, mut fnt) = (None, None, None);
    probe_config_files(&tmp, &mut acc, &mut sch, &mut fnt);
    let _ = std::fs::remove_dir_all(&tmp);
    assert_eq!(sch, Some(ThemeColorScheme::Light));
    assert_eq!(acc, Some((51, 204, 255)));
    assert_eq!(fnt, Some(GlyphFontStyle::CyberSharp));

    // Verify paint_transition preserves non-black Light theme background on empty space cells
    let light_cell = TerminalCell {
        ch: ' ',
        fg: (248, 248, 242),
        bg: (10, 18, 34),
        bold: false,
    };
    let mut out_light = vec![TerminalCell::default(); 8];
    paint_transition(
        &[light_cell; 4],
        &[light_cell; 4],
        &mut out_light,
        0.5,
        2,
        2,
        7,
    );
    assert!(out_light.iter().all(|c| c.bg == (10, 18, 34)));
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
#[cfg(feature = "product")]
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
    assert_eq!(
        viewport_buffer_dimensions(1920, 1080, 120, 36, 120),
        (960, 576)
    );
    assert_eq!(
        viewport_buffer_dimensions(3840, 2160, 120, 36, 120),
        (1920, 1152)
    );
    // A compositor-reported scale above 100% sizes the buffer with the
    // standard fractional-scale rounding instead of the 4K heuristic.
    assert_eq!(
        viewport_buffer_dimensions(1920, 1080, 120, 36, 150),
        (1200, 720)
    );
    assert_eq!(
        viewport_buffer_dimensions(1920, 1080, 120, 36, 240),
        (1920, 1152)
    );

    let b_block = get_glyph_bitmap_for_style('█', GlyphFontStyle::ArcadeBlock);
    let b_sharp = get_glyph_bitmap_for_style('█', GlyphFontStyle::CyberSharp);
    let b_round = get_glyph_bitmap_for_style('█', GlyphFontStyle::RetroRound);
    assert_ne!(b_block, b_sharp);
    assert_ne!(b_block, b_round);
}
