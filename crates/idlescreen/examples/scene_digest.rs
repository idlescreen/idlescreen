// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Deterministic scene-frame digest probe for the `.oo` parity suite.
//!
//! Drives one built-in scene through the same `EngineCtx` path the daemon
//! uses (virtual clock, seeded RNG, fixed canvas) and prints an FNV-1a
//! digest of the painted cell grid after every scene step. The `.oo` port
//! of a scene must reproduce the same digest stream bit-for-bit.
//!
//! Usage: scene_digest <scene> [cols] [rows] [seed] [fps] [max_frames] [art]
//!
//! Defaults match the Rust test harness: 80x24, seed 42, 60 fps, art
//! "IDLESCREEN\nSCREENSAVER". Output lines are `frame\t<idx>\t<hash16>`
//! followed by `done\t<frames>\t<composite16>`.

use std::fmt::Write as _;

use idlescreen::ascii::effects::EffectKind;
use idlescreen::ascii::scenes;
use idlescreen::cell::TerminalCell;
use ttfx::engine::canvas::Anchor;
use ttfx::engine::ctx::{Clock, EngineCtx};
use ttfx::engine::effect::Effect;
use ttfx::engine::terminal::TerminalConfig;
use ttfx::utils::rng::Rng;

const DEFAULT_ART: &str = "IDLESCREEN\nSCREENSAVER";
const DEFAULT_FG: (u8, u8, u8) = (248, 248, 242);
const DEFAULT_BG: (u8, u8, u8) = (0, 0, 0);

fn fnv1a_update(hash: u64, bytes: &[u8]) -> u64 {
    let mut h = hash;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

fn cell_record(cell: &TerminalCell, out: &mut String) {
    let _ = write!(
        out,
        "{},{},{},{},{},{},{},{};",
        cell.ch as u32,
        cell.fg.0,
        cell.fg.1,
        cell.fg.2,
        cell.bg.0,
        cell.bg.1,
        cell.bg.2,
        u8::from(cell.bold),
    );
}

fn grid_digest(grid: &[TerminalCell], cols: usize, rows: usize) -> u64 {
    let mut buf = String::with_capacity(cols * rows * 24);
    for y in 0..rows {
        for x in 0..cols {
            let idx = y * cols + x;
            if let Some(cell) = grid.get(idx) {
                cell_record(cell, &mut buf);
            }
        }
    }
    fnv1a_update(0xcbf2_9ce4_8422_2325, buf.as_bytes())
}

fn parse_args() -> Option<(String, usize, usize, u64, u32, u64, String)> {
    let mut it = std::env::args().skip(1);
    let scene = it.next()?;
    let cols = it.next().and_then(|s| s.parse().ok()).unwrap_or(80);
    let rows = it.next().and_then(|s| s.parse().ok()).unwrap_or(24);
    let seed = it.next().and_then(|s| s.parse().ok()).unwrap_or(42);
    let fps = it.next().and_then(|s| s.parse().ok()).unwrap_or(60);
    let max_frames = it.next().and_then(|s| s.parse().ok()).unwrap_or(4096);
    let art = it.next().unwrap_or_else(|| DEFAULT_ART.to_string());
    Some((scene, cols, rows, seed, fps, max_frames, art))
}

fn main() {
    // Pin the theme inputs so the digest is machine-independent: the neon
    // overdrive path maps pure black to the system accent colour.
    // SAFETY: `main` runs before any threads are spawned, so no concurrent
    // `getenv` can observe a partially-updated environment.
    unsafe {
        std::env::set_var("IDLE_THEME_ACCENT", "5196DA");
        std::env::set_var("IDLE_THEME_MODE", "dark");
        std::env::set_var("IDLE_FONT_STYLE", "arcade");
    }

    let Some((scene, cols, rows, seed, fps, max_frames, art)) = parse_args() else {
        eprintln!("usage: scene_digest <scene> [cols] [rows] [seed] [fps] [max_frames] [art]");
        std::process::exit(2);
    };
    let Some(kind) = EffectKind::parse(&scene) else {
        eprintln!("ERR\tscene\tunknown scene: {scene}");
        std::process::exit(2);
    };

    let config = TerminalConfig {
        canvas_width: cols as i64,
        canvas_height: rows as i64,
        ignore_terminal_dimensions: true,
        anchor_text: Anchor::C,
        frame_rate: i64::from(fps),
        ..Default::default()
    };
    let clock = Clock::virtual_with_frame_rate(i64::from(fps));
    let rng = Rng::seeded(seed);
    let Ok(mut ctx) = EngineCtx::new(&art, config, rng, clock) else {
        eprintln!("ERR\tscene\tEngineCtx::new failed for {scene}");
        std::process::exit(1);
    };
    let mut effect: Box<dyn Effect> = scenes::create_scene(kind);
    if effect.build(&mut ctx).is_err() {
        eprintln!("ERR\tscene\tbuild failed for {scene}");
        std::process::exit(1);
    }

    let mut grid = vec![TerminalCell::default(); cols * rows];

    // `dump` mode: emit the canvas geometry and per-cell build data so the
    // `.oo` port can replicate `build_canvas_cells` for the fixed probe
    // configuration without re-deriving ttfx's layout engine.
    if std::env::var("SCENE_DIGEST_DUMP").is_ok() {
        let c = &ctx.terminal.canvas;
        println!(
            "canvas\tleft={}\tright={}\ttop={}\tbottom={}\ttext_l={}\ttext_r={}\ttext_t={}\ttext_b={}",
            c.left, c.right, c.top, c.bottom, c.text_left, c.text_right, c.text_top, c.text_bottom
        );
        println!(
            "visible\tleft={}\tright={}\ttop={}\tbottom={}\troff={}\tcoff={}",
            ctx.terminal.visible_left,
            ctx.terminal.visible_right,
            ctx.terminal.visible_top,
            ctx.terminal.visible_bottom,
            ctx.terminal.canvas_row_offset,
            ctx.terminal.canvas_column_offset,
        );
        let filter = ttfx::engine::terminal::CharacterFilter {
            input_chars: true,
            inner_fill_chars: true,
            outer_fill_chars: true,
            added_chars: false,
        };
        let ids = ctx.terminal.get_characters(
            &mut ctx.rng,
            filter,
            ttfx::engine::terminal::CharacterSort::TopToBottomLeftToRight,
        );
        for id in ids {
            let ch = &ctx.terminal.arena[id.0 as usize];
            let co = ch.input_coord;
            println!(
                "cell\tcol={}\trow={}\tfill={}\tsym={:?}\tpre={}",
                co.column,
                co.row,
                ch.is_fill_character || ch.input_symbol == " ",
                ch.input_symbol,
                ch.uses_input_preexisting_colors,
            );
        }
        return;
    }

    let mut composite = 0xcbf2_9ce4_8422_2325u64;
    let mut frames: u64 = 0;
    while frames < max_frames {
        let Some(frame) = effect.next_frame(&mut ctx) else {
            break;
        };
        idlescreen::ascii::draw::paint_frame(&frame, &mut grid, cols, rows, DEFAULT_FG, DEFAULT_BG);
        let digest = grid_digest(&grid, cols, rows);
        composite = fnv1a_update(composite, digest.to_le_bytes().as_slice());
        // Signed-i64 decimal: bit-identical to the .oo Int hash output.
        println!("frame\t{frames}\t{}", digest as i64);
        if std::env::var("SCENE_DIGEST_GRID").is_ok_and(|v| v == frames.to_string()) {
            let mut buf = String::new();
            for (idx, cell) in grid.iter().enumerate() {
                buf.clear();
                cell_record(cell, &mut buf);
                println!("cell\t{idx}\t{buf}");
            }
        }
        frames += 1;
    }
    println!("done\t{frames}\t{}", composite as i64);
}
