// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::layout::{LayoutTier, is_portrait, render_banner};
use super::super::state::LruFontAtlasCache;
use super::super::sys_info::{clean_desktop_env, clean_kernel_version, clean_os_name};
use super::super::unicode::{
    GridCellChar, normalize_to_ascii_fold, place_unicode_str, string_width,
};

#[test]
fn test_clean_os_name() {
    assert_eq!(
        clean_os_name("Fedora Linux 40 (Workstation Edition)"),
        "FEDORA"
    );
    assert_eq!(clean_os_name("Ubuntu 24.04 LTS"), "UBUNTU");
    assert_eq!(clean_os_name("Debian GNU/Linux 12 (bookworm)"), "DEBIAN");
    assert_eq!(clean_os_name("Arch Linux"), "ARCH");
    assert_eq!(clean_os_name("Alpine Linux v3.21 (musl)"), "ALPINE");
    assert_eq!(clean_os_name("openSUSE Tumbleweed"), "OPENSUSE");
    assert_eq!(clean_os_name("SUSE Linux Enterprise Server 15 SP6"), "SLES");
    assert_eq!(clean_os_name("Parch Linux"), "PARCH");
    assert_eq!(clean_os_name("Archcraft"), "ARCHCRAFT");
    assert_eq!(clean_os_name("ArcoLinux"), "ARCO");
    assert_eq!(clean_os_name("Linux"), "LINUX");
    assert_eq!(clean_os_name("GNU/Linux"), "LINUX");
    assert_eq!(clean_os_name(""), "IDLESCREEN");
    assert_eq!(clean_os_name("unknown"), "IDLESCREEN");
}

#[test]
fn test_clean_desktop_env() {
    assert_eq!(clean_desktop_env("GNOME"), "GNOME");
    assert_eq!(clean_desktop_env("GNOME:Mutter"), "GNOME");
    assert_eq!(clean_desktop_env("mutter"), "GNOME");
    assert_eq!(clean_desktop_env("KDE Plasma"), "KDE");
    assert_eq!(clean_desktop_env("kwin_wayland"), "KDE");
    assert_eq!(clean_desktop_env("Hyprland"), "HYPRLAND");
    assert_eq!(clean_desktop_env("Sway"), "SWAY");
    assert_eq!(clean_desktop_env("default"), "DESKTOP");
    assert_eq!(clean_desktop_env(""), "DESKTOP");
}

#[test]
fn test_clean_kernel_version() {
    assert_eq!(clean_kernel_version("Linux 6.13.5-arch1-1"), "LINUX");
    assert_eq!(clean_kernel_version(""), "LINUX");
}

#[test]
fn test_unicode_nfkd_fold() {
    assert_eq!(normalize_to_ascii_fold("Éléphant"), "Elephant");
    assert_eq!(normalize_to_ascii_fold("München"), "Munchen");
    assert_eq!(normalize_to_ascii_fold("Česká"), "Ceska");
}

#[test]
fn test_cjk_width_handling() {
    let text = "你好";
    assert_eq!(string_width(text), 4);
    let cells = place_unicode_str(text, 10);
    assert_eq!(cells.len(), 4);
    assert_eq!(cells[0], GridCellChar::WideLeading('你'));
    assert_eq!(cells[1], GridCellChar::WideContinuation);
    assert_eq!(cells[2], GridCellChar::WideLeading('好'));
    assert_eq!(cells[3], GridCellChar::WideContinuation);
}

#[test]
fn test_3_tier_layout() {
    assert_eq!(LayoutTier::for_dimensions(80, 24), LayoutTier::Wide);
    assert_eq!(LayoutTier::for_dimensions(40, 24), LayoutTier::Medium);
    assert_eq!(LayoutTier::for_dimensions(20, 24), LayoutTier::Narrow);

    let wide_banner = render_banner("IDLE", None, 80, 24);
    assert_eq!(wide_banner.height, 5);

    let med_banner = render_banner("IDLE", None, 40, 24);
    assert_eq!(med_banner.height, 3);

    let narrow_banner = render_banner("IDLE", None, 15, 24);
    assert!(narrow_banner.width <= 1);
}

#[test]
fn test_portrait_detection() {
    assert!(is_portrait(30, 60)); // 60/30 = 2.0 > 1.4
    assert!(!is_portrait(80, 24)); // 24/80 = 0.3 < 1.4
}

#[test]
fn test_lru_font_atlas_cache() {
    let mut cache = LruFontAtlasCache::new(3);
    let b1 = cache.get_or_render("OS1", None, 80, 24);
    let b2 = cache.get_or_render("OS2", None, 80, 24);
    let b3 = cache.get_or_render("OS3", None, 80, 24);

    assert_eq!(b1.lines.len(), 5);
    assert_eq!(b2.lines.len(), 5);
    assert_eq!(b3.lines.len(), 5);

    // Fetching OS1 makes it MRU
    let b1_again = cache.get_or_render("OS1", None, 80, 24);
    assert_eq!(b1_again.lines, b1.lines);

    // Inserting OS4 should evict OS2
    let _b4 = cache.get_or_render("OS4", None, 80, 24);
    assert!(cache.get(&("OS2".to_string(), 80, 24)).is_none());
    assert!(cache.get(&("OS1".to_string(), 80, 24)).is_some());
}

#[test]
fn test_banner_responsive_scaling_up_to_400_percent() {
    // 14" laptop screen fullscreen display (1920x1080 -> 240 cols, 67 rows)
    // Short brand text scales to 400% (20 rows tall)
    let banner_400 = render_banner("UBUNTU", None, 240, 67);
    assert_eq!(banner_400.height, 20);
    assert!(banner_400.width <= 240);

    // 10-char word on 240x67 scales to 400% (20 rows) if width permits, or 300% (15 rows)
    let banner_idle = render_banner("IDLESCREEN", None, 240, 67);
    assert!(banner_idle.height == 20 || banner_idle.height == 15);
    assert!(banner_idle.width <= 240);

    // With subtitle: height adds subtitle rows (20 + 2 = 22 rows)
    let banner_with_sub = render_banner("UBUNTU", Some("v4.1.7"), 240, 67);
    assert_eq!(banner_with_sub.height, 22);

    // Medium-large window (120 cols, 35 rows) scales to 15 rows (300%) or 10 rows (200%)
    let banner_med_large = render_banner("ARCH", None, 120, 35);
    assert_eq!(banner_med_large.height, 15);

    // Standard 80x24 terminal preserves 5 rows (100%)
    let banner_std = render_banner("IDLE", None, 80, 24);
    assert_eq!(banner_std.height, 5);

    // Edge case: empty string
    let banner_empty = render_banner("", None, 240, 67);
    assert_eq!(banner_empty.height, 5);

    // Edge case: very long text that cannot fit at scale 4 falls back to lower scale
    let banner_long = render_banner(
        "A VERY LONG PHRASE THAT CANNOT FIT AT SCALE 4",
        None,
        120,
        35,
    );
    assert!(banner_long.height <= 15);

    // Verify uniform line width across all rows of the scaled banner
    let first_w = banner_400.lines[0].chars().count();
    for line in &banner_400.lines {
        assert_eq!(
            line.chars().count(),
            first_w,
            "every row should have identical character width"
        );
    }
    assert_eq!(first_w, banner_400.width);

    // Verify uniform character width for A, B, D
    let b_a = render_banner("A", None, 80, 24);
    let b_b = render_banner("B", None, 80, 24);
    let b_d = render_banner("D", None, 80, 24);
    assert_eq!(b_a.width, 5);
    assert_eq!(b_b.width, 5);
    assert_eq!(b_d.width, 5);
}

#[test]
fn test_clean_os_name_superfedora() {
    assert_eq!(clean_os_name("SuperFedora"), "SUPERFEDORA");
    assert_eq!(clean_os_name("Super Fedora 40"), "SUPERFEDORA");
    assert_eq!(clean_os_name("SUPERFEDORA"), "SUPERFEDORA");
}

#[test]
fn test_is_superfedora_machine_probe() {
    // Test with explicit environment override
    // SAFETY: tests run single-threaded per-process or set scoped env
    unsafe {
        std::env::set_var("IDLE_SUPERFEDORA", "1");
    }
    assert!(super::super::sys_info::is_superfedora_machine());
    unsafe {
        std::env::remove_var("IDLE_SUPERFEDORA");
    }
}

#[test]
fn test_superfedora_banner_stacked_rendering() {
    // Wide tier: 22 lines from website pixel art (6 for SUPER, 1 blank divider, 15 for FEDORA)
    // Requires cols >= 90 and rows >= 22
    let wide = render_banner("SUPERFEDORA", None, 120, 30);
    assert_eq!(wide.height, 22);
    assert_eq!(wide.width, 90);
    // SUPER is positioned in upper-left (starts with 3 spaces in row 0)
    assert!(wide.lines[0].starts_with("   █████"));
    // Row 6 is blank divider
    assert!(wide.lines[6].is_empty());
    // FEDORA starts at col 0 in row 7
    assert!(wide.lines[7].starts_with("████████████"));

    println!("\n--- WIDE TIER (120x30) ---");
    for l in &wide.lines {
        println!("{l}");
    }

    // Wide tier with subtitle (requires rows >= 24)
    let wide_sub = render_banner("SUPERFEDORA", Some("WORKSTATION"), 120, 30);
    assert_eq!(wide_sub.height, 24);
    assert!(wide_sub.lines.last().unwrap().contains("WORKSTATION"));

    // Standard 80x24 terminal: below 90 cols threshold, drops to Medium tier gracefully
    let std_term = render_banner("SUPERFEDORA", None, 80, 24);
    assert_eq!(std_term.height, 7);
    assert_eq!(std_term.width, 23);
    assert!(std_term.lines[0].starts_with("█▀▀")); // Left-aligned at upper-left of FEDORA

    // Medium tier: 7 lines (3 for SUPER, 1 blank, 3 for FEDORA) positioned upper-left
    let med = render_banner("SUPERFEDORA", None, 40, 24);
    assert_eq!(med.height, 7);
    assert_eq!(med.width, 23);
    assert!(med.lines[0].starts_with("█▀▀")); // Left-aligned at upper-left of FEDORA
    assert!(med.lines[3].is_empty()); // Blank divider line

    println!("\n--- MEDIUM TIER (40x24) ---");
    for l in &med.lines {
        println!("{l}");
    }

    // Narrow tier: vertical stack
    let narrow = render_banner("SUPERFEDORA", None, 15, 24);
    assert_eq!(narrow.width, 1);
    assert!(narrow.height >= 10);
    assert_eq!(narrow.lines[0], "S");
    assert_eq!(narrow.lines[1], "U");
}

#[test]
fn test_ascii_new_superfedora() {
    let mut saver = super::super::Ascii::new_superfedora();
    assert_eq!(saver.fg, (81, 162, 218)); // Fedora Cyan
    assert_eq!(saver.bg, (0, 0, 0));
    assert_eq!(saver.effect, super::super::effects::EffectKind::Spotlights);
    assert_eq!(saver.rotation.targets, vec!["superfedora".to_string()]);

    // Verify it initializes, loads the website banner, and steps frames cleanly at fullscreen (240x67)
    saver.init(240, 67);
    assert!(saver.art_text.contains("FEDORA") || saver.art_text.contains("████"));
    let mut grid_wide = vec![crate::cell::TerminalCell::default(); 240 * 67];
    for _ in 0..5 {
        saver.update(std::time::Duration::from_millis(16), 240, 67);
        saver.draw(&mut grid_wide, 240, 67);
    }

    // Also verify standard 80x24 terminal gracefully draws Medium tier without clipping
    saver.init(80, 24);
    assert!(saver.art_text.contains("FEDORA") || saver.art_text.contains("█▀▀"));
    let mut grid = vec![crate::cell::TerminalCell::default(); 80 * 24];
    for _ in 0..5 {
        saver.update(std::time::Duration::from_millis(16), 80, 24);
        saver.draw(&mut grid, 80, 24);
    }
}
