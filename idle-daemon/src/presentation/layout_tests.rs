// SPDX-License-Identifier: MIT

use wayland_present::OutputLayout;

use super::*;

fn layout(id: u32, x: i32, y: i32, w: u32, h: u32) -> OutputLayout {
    OutputLayout {
        id,
        width: w,
        height: h,
        refresh_rate_hz: 60,
        x,
        y,
        scale: 1,
    }
}

#[test]
fn virtual_desktop_single_output_zero_origin() {
    let layouts = vec![layout(1, 0, 0, 1920, 1080)];
    let (_, _, width, height) = virtual_desktop(&layouts);
    assert_eq!(width, 1920);
    assert_eq!(height, 1080);
}

#[test]
fn virtual_desktop_two_outputs_side_by_side() {
    let layouts = vec![layout(1, 0, 0, 1920, 1080), layout(2, 1920, 0, 1920, 1080)];
    let (min_x, min_y, width, height) = virtual_desktop(&layouts);
    assert_eq!(min_x, 0);
    assert_eq!(min_y, 0);
    assert_eq!(width, 3840);
    assert_eq!(height, 1080);
}

#[test]
fn virtual_desktop_handles_negative_origin() {
    let layouts = vec![layout(1, -1920, 0, 1920, 1080), layout(2, 0, 0, 1920, 1080)];
    let (min_x, _min_y, width, height) = virtual_desktop(&layouts);
    assert_eq!(min_x, -1920);
    assert_eq!(width, 3840);
    assert_eq!(height, 1080);
}

#[test]
fn virtual_desktop_empty_layouts_is_unit() {
    let (_, _, width, height) = virtual_desktop(&[]);
    assert_eq!(width, 1);
    assert_eq!(height, 1);
}

#[test]
fn normalize_layout_positions_noop_for_single_output() {
    let mut layouts = vec![layout(1, 0, 0, 1920, 1080)];
    normalize_layout_positions(&mut layouts);
    assert_eq!(layouts[0].x, 0);
    assert_eq!(layouts[0].y, 0);
}

#[test]
fn normalize_layout_positions_noop_when_already_offset() {
    let mut layouts = vec![
        layout(1, 100, 50, 1920, 1080),
        layout(2, 2020, 50, 1920, 1080),
    ];
    normalize_layout_positions(&mut layouts);
    assert_eq!(layouts[0].x, 100);
    assert_eq!(layouts[1].x, 2020);
}

#[test]
fn normalize_layout_positions_stacks_unset_outputs() {
    let mut layouts = vec![
        layout(1, 0, 0, 1920, 1080),
        layout(2, 0, 0, 1280, 720),
        layout(3, 0, 0, 800, 600),
    ];
    normalize_layout_positions(&mut layouts);
    assert_eq!(layouts[0].x, 0);
    assert_eq!(layouts[1].x, 1920);
    assert_eq!(layouts[2].x, 3200);
    for entry in &layouts {
        assert_eq!(entry.y, 0);
    }
}

#[test]
fn monitor_cell_bounds_full_extent() {
    let primary = layout(1, 0, 0, 3840, 1080);
    let b = monitor_cell_bounds(primary, 0, 0, 3840, 1080, 200, 50, true);
    assert_eq!(b.start_col, 0);
    assert_eq!(b.end_col, 200);
    assert_eq!(b.start_row, 0);
    assert_eq!(b.end_row, 50);
    assert!(b.is_primary);
}

#[test]
fn monitor_cell_bounds_left_half() {
    let primary = layout(1, 0, 0, 1920, 1080);
    let b = monitor_cell_bounds(primary, 0, 0, 3840, 1080, 200, 50, true);
    assert_eq!(b.start_col, 0);
    assert_eq!(b.end_col, 100);
    assert_eq!(b.start_row, 0);
    assert_eq!(b.end_row, 50);
}

#[test]
fn monitor_cell_bounds_right_half() {
    let primary = layout(1, 1920, 0, 1920, 1080);
    let b = monitor_cell_bounds(primary, 0, 0, 3840, 1080, 200, 50, true);
    assert_eq!(b.start_col, 100);
    assert_eq!(b.end_col, 200);
    assert_eq!(b.start_row, 0);
    assert_eq!(b.end_row, 50);
}

#[test]
fn monitor_cell_bounds_negative_origin_offsets() {
    let primary = layout(1, -1920, 0, 1920, 1080);
    let b = monitor_cell_bounds(primary, -1920, 0, 3840, 1080, 200, 50, true);
    assert_eq!(b.start_col, 0);
    assert_eq!(b.end_col, 100);
}

#[test]
fn primary_bounds_in_grid_returns_primary_true() {
    let primary = layout(7, 0, 0, 1920, 1080);
    let b = primary_bounds_in_grid(primary, 0, 0, 1920, 1080, 100, 50);
    assert!(b.is_primary);
    assert_eq!(b.end_col, 100);
    assert_eq!(b.end_row, 50);
}

#[test]
fn virtual_desktop_saturates_huge_width() {
    // width near u32::MAX must not panic or wrap span to zero via i32 cast.
    let layouts = vec![layout(1, 0, 0, u32::MAX, 1080)];
    let (_, _, width, height) = virtual_desktop(&layouts);
    assert!(width >= 1);
    assert_eq!(height, 1080);
}

#[test]
fn virtual_desktop_saturates_huge_height() {
    let layouts = vec![layout(1, 0, 0, 100, u32::MAX)];
    let (_, _, width, height) = virtual_desktop(&layouts);
    assert_eq!(width, 100);
    assert!(height >= 1);
}

#[test]
fn virtual_desktop_saturates_when_x_near_i32_max() {
    // x + width would overflow i32 without saturating_add.
    let layouts = vec![layout(1, i32::MAX - 10, 0, 1000, 100)];
    let (min_x, _, width, height) = virtual_desktop(&layouts);
    assert_eq!(min_x, i32::MAX - 10);
    assert!(width >= 1);
    assert_eq!(height, 100);
}

#[test]
fn virtual_desktop_stacked_vertical_span() {
    let layouts = vec![layout(1, 0, 0, 800, 600), layout(2, 0, 600, 800, 600)];
    let (min_x, min_y, width, height) = virtual_desktop(&layouts);
    assert_eq!(min_x, 0);
    assert_eq!(min_y, 0);
    assert_eq!(width, 800);
    assert_eq!(height, 1200);
}

#[test]
fn monitor_cell_bounds_zero_total_does_not_panic() {
    let primary = layout(1, 0, 0, 100, 100);
    let b = monitor_cell_bounds(primary, 0, 0, 0, 0, 10, 10, true);
    // total dims clamped to 1 for division; result is finite.
    assert!(b.end_col <= 10 || b.start_col <= 10);
}

#[test]
fn monitor_cell_bounds_clamps_negative_relative_to_zero_cols() {
    // layout.x < min_x → saturating_sub yields negative rel; nonneg_usize → 0.
    let primary = layout(1, -100, -50, 200, 100);
    let b = monitor_cell_bounds(primary, 0, 0, 1000, 1000, 100, 50, false);
    assert_eq!(b.start_col, 0);
    assert_eq!(b.start_row, 0);
    assert!(!b.is_primary);
}

#[test]
fn normalize_layout_positions_saturates_wide_chain() {
    let mut layouts = vec![
        layout(1, 0, 0, 2_000_000_000, 100),
        layout(2, 0, 0, 2_000_000_000, 100),
    ];
    normalize_layout_positions(&mut layouts);
    assert_eq!(layouts[0].x, 0);
    // Second x is saturating_add of first width as i32 (clamped).
    assert!(layouts[1].x > 0 || layouts[1].x == i32::MAX);
}

#[test]
fn normalize_layout_positions_saturates_max_u32_widths() {
    let mut layouts = vec![
        layout(1, 0, 0, u32::MAX, 10),
        layout(2, 0, 0, u32::MAX, 10),
        layout(3, 0, 0, u32::MAX, 10),
    ];
    normalize_layout_positions(&mut layouts);
    assert_eq!(layouts[0].x, 0);
    // extent_i32(u32::MAX) == i32::MAX; next adds saturate at i32::MAX.
    assert_eq!(layouts[1].x, i32::MAX);
    assert_eq!(layouts[2].x, i32::MAX);
}

#[test]
fn monitor_cell_bounds_end_not_before_start_on_tiny_grid() {
    let primary = layout(1, 0, 0, 1, 1);
    let b = monitor_cell_bounds(primary, 0, 0, 1, 1, 1, 1, true);
    assert!(b.end_col >= b.start_col);
    assert!(b.end_row >= b.start_row);
}
