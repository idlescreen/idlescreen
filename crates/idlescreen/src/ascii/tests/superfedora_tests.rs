// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

#![allow(unsafe_code)]

use super::super::Ascii;
use super::super::SceneStep;
use super::super::effects::EffectKind;
use super::super::layout::render_banner;
use super::super::sys_info::clean_os_name;
use crate::cell::TerminalCell;
use std::time::Duration;

const COLS: usize = 80;
const ROWS: usize = 24;

/// The time one display frame stands for, as the presenter drives it.
const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

#[test]
fn test_clean_os_name_superfedora() {
    assert_eq!(clean_os_name("SuperFedora"), "SUPERFEDORA");
    assert_eq!(clean_os_name("Super Fedora 40"), "SUPERFEDORA");
    assert_eq!(clean_os_name("SUPERFEDORA"), "SUPERFEDORA");
}

#[test]
fn test_is_superfedora_machine_probe() {
    super::super::sys_info::set_superfedora_override(Some(true));
    assert!(super::super::sys_info::is_superfedora_machine());
    super::super::sys_info::set_superfedora_override(None);
    // The override is thread-scoped: it must not leak into other threads.
    assert!(
        std::thread::spawn(super::super::sys_info::is_superfedora_machine)
            .join()
            .is_ok()
    );
}

#[test]
fn test_superfedora_banner_stacked_rendering() {
    let wide = render_banner("SUPERFEDORA", None, 120, 30);
    assert_eq!(wide.height, 22);
    assert_eq!(wide.width, 90);
    assert!(wide.lines[0].starts_with("   █████"));
    assert!(wide.lines[6].is_empty());
    assert!(wide.lines[7].starts_with("████████████"));

    let wide_sub = render_banner("SUPERFEDORA", Some("WORKSTATION"), 120, 30);
    assert_eq!(wide_sub.height, 24);
    assert!(wide_sub.lines.last().unwrap().contains("WORKSTATION"));

    let std_term = render_banner("SUPERFEDORA", None, 80, 24);
    assert_eq!(std_term.height, 7);
    assert_eq!(std_term.width, 23);
    assert!(std_term.lines[0].starts_with("█▀▀"));

    let med = render_banner("SUPERFEDORA", None, 40, 24);
    assert_eq!(med.height, 7);
    assert_eq!(med.width, 23);
    assert!(med.lines[0].starts_with("█▀▀"));
    assert!(med.lines[3].is_empty());

    let narrow = render_banner("SUPERFEDORA", None, 15, 24);
    assert_eq!(narrow.width, 1);
    assert!(narrow.height >= 10);
    assert_eq!(narrow.lines[0], "S");
    assert_eq!(narrow.lines[1], "U");
}

#[test]
fn test_ascii_new_superfedora() {
    super::super::sys_info::set_superfedora_override(Some(true));

    let mut saver = Ascii::new_superfedora();
    assert_eq!(saver.fg, (81, 162, 218));
    assert_eq!(saver.bg, (0, 0, 0));
    assert!(EffectKind::ALL.contains(&saver.effect));
    assert_eq!(saver.rotation.targets, vec!["superfedora".to_string()]);
    assert_eq!(saver.rotation.active_text(), "SUPERFEDORA");

    saver.init(120, 36);
    assert!(saver.art_text.contains("████"));
    let mut grid_wide = vec![TerminalCell::default(); 120 * 36];
    for _ in 0..5 {
        saver.update(Duration::from_millis(16), 120, 36);
        saver.draw(&mut grid_wide, 120, 36);
    }
    assert!(
        grid_wide
            .iter()
            .all(|c| super::super::draw::is_pixel_art_char(c.ch)),
        "all rendered cells must be pure pixel-art block elements"
    );

    saver.init(80, 24);
    assert!(saver.art_text.contains("█▀▀"));
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    for _ in 0..5 {
        saver.update(Duration::from_millis(16), 80, 24);
        saver.draw(&mut grid, 80, 24);
    }
    assert!(
        grid.iter()
            .all(|c| super::super::draw::is_pixel_art_char(c.ch)),
        "all rendered cells in 80x24 must be pure pixel-art block elements"
    );

    // The override only applies to this thread, so clearing it here leaves
    // every other test unaffected.
    super::super::sys_info::set_superfedora_override(None);
}

#[test]
fn superfedora_rotates_scenes_past_dwell_without_pinning() {
    let mut saver = Ascii::new_superfedora();
    assert!(!saver.pinned);
    let initial_effect = saver.effect;
    assert!(EffectKind::ALL.contains(&initial_effect));
    saver.init(COLS, ROWS);

    saver.engine = None;
    saver.dwell_left = 0.0;
    saver.update(Duration::from_millis(50), COLS, ROWS);
    assert!(saver.transition_active);

    for _ in 0..10 {
        saver.update(Duration::from_millis(100), COLS, ROWS);
    }
    assert_ne!(saver.effect, initial_effect);
}

#[test]
fn superfedora_rotates_through_all_53_distinct_scenes_and_randomizes_start() {
    let mut saver = Ascii::new_superfedora();
    assert!(EffectKind::ALL.contains(&saver.effect));
    let mut seen = std::collections::HashSet::new();
    seen.insert(saver.effect);
    for _ in 1..53 {
        saver.cycle_effect();
        assert!(
            seen.insert(saver.effect),
            "effect {:?} was repeated before all 53 played",
            saver.effect
        );
    }
    assert_eq!(seen.len(), 53);
    let last_effect = saver.effect;
    saver.cycle_effect();
    assert_ne!(
        saver.effect, last_effect,
        "cycle boundary must not immediately repeat"
    );
    let mut initial_scenes = std::collections::HashSet::new();
    for _ in 0..32 {
        initial_scenes.insert(Ascii::new_superfedora().effect);
    }
    assert!(
        initial_scenes.len() > 5,
        "new_superfedora() must randomize initial scene, got only {initial_scenes:?}"
    );
}

#[test]
fn all_53_scenes_render_strictly_pixel_art_blocks_with_superfedora_banner() {
    let (cols, rows) = (120, 36);
    let mut saver = Ascii::new_superfedora();
    saver.init(cols, rows);
    let mut grid = vec![TerminalCell::default(); cols * rows];

    assert_eq!(EffectKind::ALL.len(), 53);
    for &kind in &EffectKind::ALL {
        saver.pin_effect(kind);
        assert_eq!(saver.rotation.active_text(), "SUPERFEDORA");
        for _ in 0..8 {
            saver.update(Duration::from_millis(16), cols, rows);
            saver.draw(&mut grid, cols, rows);
            for cell in &grid {
                assert!(
                    super::super::draw::is_pixel_art_char(cell.ch),
                    "Scene {} emitted non-pixel-art char {:?} (U+{:04X})",
                    kind.name(),
                    cell.ch,
                    cell.ch as u32
                );
                assert!(
                    cell.ch == ' ' || ('\u{2580}'..='\u{259F}').contains(&cell.ch),
                    "Scene {} emitted char outside Unicode Block Elements: {:?} (U+{:04X})",
                    kind.name(),
                    cell.ch,
                    cell.ch as u32
                );
            }
        }
        let mut frames = 8usize;
        let mut buffer = String::new();
        while let Some(ref mut s) = saver.engine {
            match s.next_frame_into(&mut buffer, FRAME) {
                SceneStep::Advanced => frames += 1,
                SceneStep::Held => {}
                SceneStep::Finished => break,
            }
        }
        assert!(
            (35..=200).contains(&frames),
            "Scene {} took {} rendered frames (expected 35..=200: a scene fills its slot)",
            kind.name(),
            frames
        );
    }
}

#[test]
fn settled_bevel_persists_into_transition_without_color_flash() {
    let (cols, rows) = (120, 36);
    let mut saver = Ascii::new_superfedora();
    saver.pin_effect(EffectKind::Wipe);
    saver.init(cols, rows);

    for _ in 0..220 {
        saver.update(FRAME, cols, rows);
        if saver.is_settled() {
            break;
        }
    }
    assert!(
        saver.is_settled(),
        "the scene should settle within its slot"
    );

    let mut settled_grid = vec![TerminalCell::default(); cols * rows];
    saver.draw(&mut settled_grid, cols, rows);

    let find_block = |y_range: std::ops::RangeInclusive<usize>| {
        settled_grid
            .iter()
            .enumerate()
            .find(|(idx, c)| c.ch == '█' && y_range.contains(&(*idx / cols)))
            .map(|(idx, _)| idx)
            .expect("should find solid block in wordmark")
    };
    let super_cell_idx = find_block(6..=12);
    let fedora_cell_idx = find_block(14..=28);

    let settled_super_fg = settled_grid[super_cell_idx].fg;
    let settled_fedora_fg = settled_grid[fedora_cell_idx].fg;

    saver.begin_transition();
    assert!(!saver.is_settled());
    assert_eq!(
        saver.transition_from_grid[super_cell_idx].fg,
        settled_super_fg
    );
    assert_eq!(
        saver.transition_from_grid[fedora_cell_idx].fg,
        settled_fedora_fg
    );
}
