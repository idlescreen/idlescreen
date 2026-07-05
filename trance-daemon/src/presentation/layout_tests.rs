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
