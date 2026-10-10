// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::super::layout::{LayoutTier, render_banner, is_portrait};
use super::super::state::LruFontAtlasCache;
use super::super::sys_info::clean_os_name;
use super::super::unicode::{normalize_to_ascii_fold, place_unicode_str, string_width, GridCellChar};

#[test]
fn test_clean_os_name() {
    assert_eq!(clean_os_name("Fedora Linux 40 (Workstation Edition)"), "Fedora Linux");
    assert_eq!(clean_os_name("Ubuntu 24.04 LTS"), "Ubuntu");
    assert_eq!(clean_os_name("Debian GNU/Linux 12 (bookworm)"), "Debian");
    assert_eq!(clean_os_name("Arch Linux"), "Arch Linux");
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
