// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Live side-by-side 16-bit arcade mini-preview viewport for wide terminals in `idlescreen tui`.

use std::fmt::Write as FmtWrite;
use std::time::Duration;

use super::model::{ModeChoice, TuiModel};
use super::view::fit_to_width;
use crate::ascii::Ascii;
use crate::ascii::effects::EffectKind;
use crate::ascii::state::TextMode;
use crate::cell::TerminalCell;

pub const WIDE_TERM_MIN_COLS: usize = 105;
pub const MINI_PREVIEW_COL: usize = 75;
pub const MINI_PREVIEW_ROWS: usize = 16;

pub struct MiniPreview {
    pub saver: Ascii,
    pub cells: Vec<TerminalCell>,
    pub active_saver: String,
    pub mode: ModeChoice,
    pub custom_text: String,
    pub dwell_secs: f32,
    pub cols: usize,
    pub rows: usize,
}

impl MiniPreview {
    pub fn canvas_cols_for_terminal(term_cols: usize) -> usize {
        if term_cols < WIDE_TERM_MIN_COLS {
            0
        } else {
            term_cols.saturating_sub(MINI_PREVIEW_COL + 2).clamp(28, 48)
        }
    }

    pub fn effective_cols(term_cols: usize) -> usize {
        let c = Self::canvas_cols_for_terminal(term_cols);
        if c == 0 { 44 } else { c }
    }

    pub fn new(model: &TuiModel, term_cols: usize) -> Self {
        let cols = Self::effective_cols(term_cols);
        let rows = MINI_PREVIEW_ROWS;
        let mut saver = build_mini_saver(model);
        saver.init(cols, rows);
        let mut cells = vec![TerminalCell::default(); cols * rows];
        saver.update(Duration::from_millis(120), cols, rows);
        saver.draw(&mut cells, cols, rows);
        Self {
            saver,
            cells,
            active_saver: model.active_saver_name().to_string(),
            mode: model.mode,
            custom_text: model.custom_text.clone(),
            dwell_secs: model.dwell_secs,
            cols,
            rows,
        }
    }

    pub fn sync_with_model(&mut self, model: &TuiModel, term_cols: usize) {
        let target_cols = Self::effective_cols(term_cols);
        let text_changed =
            model.mode == ModeChoice::CustomText && self.custom_text != model.custom_text;
        let changed = self.active_saver != model.active_saver_name()
            || self.mode != model.mode
            || text_changed
            || (self.dwell_secs - model.dwell_secs).abs() > f32::EPSILON
            || self.cols != target_cols;

        if changed {
            self.active_saver = model.active_saver_name().to_string();
            self.mode = model.mode;
            self.custom_text.clone_from(&model.custom_text);
            self.dwell_secs = model.dwell_secs;
            self.cols = target_cols;
            self.rows = MINI_PREVIEW_ROWS;
            self.saver = build_mini_saver(model);
            self.saver.init(self.cols, self.rows);
            self.cells
                .resize(self.cols * self.rows, TerminalCell::default());
            self.cells.fill(TerminalCell::default());
            self.saver
                .update(Duration::from_millis(80), self.cols, self.rows);
            self.saver.draw(&mut self.cells, self.cols, self.rows);
        }
    }

    pub fn step(&mut self, dt: Duration) {
        self.saver.update(dt, self.cols, self.rows);
        self.saver.draw(&mut self.cells, self.cols, self.rows);
    }

    pub fn is_settled(&self) -> bool {
        self.saver.is_settled()
    }

    pub fn render_into(&self, model: &TuiModel, term_cols: usize, b: &mut String) {
        let canvas_cols = Self::canvas_cols_for_terminal(term_cols).min(self.cols);
        if canvas_cols == 0 {
            return;
        }
        let col = MINI_PREVIEW_COL;
        let title = " Live Preview ";
        let rem_dashes = canvas_cols.saturating_sub(title.len() + 1);
        let top_bar: String = std::iter::repeat_n('─', rem_dashes).collect();
        let _ = write!(b, "\x1b[1;{col}H\x1b[1;36m┌─{title}{top_bar}┐\x1b[0m");

        let scene_name = self.saver.effect.name();
        let hdr = format!(" Scene: {scene_name} │ {}", model.sequence_summary());
        let hdr_fit = fit_to_width(&hdr, 0, canvas_cols, false);
        let _ = write!(
            b,
            "\x1b[2;{col}H\x1b[1;36m│\x1b[0m\x1b[1;33m{hdr_fit}\x1b[0m\x1b[1;36m│\x1b[0m"
        );

        for y in 0..self.rows {
            let term_row = 3 + y;
            let _ = write!(b, "\x1b[{term_row};{col}H\x1b[1;36m│\x1b[0m");
            let mut cur_fg: Option<(u8, u8, u8)> = None;
            let mut cur_bg: Option<(u8, u8, u8)> = None;
            let mut cur_bold: Option<bool> = None;
            let row_offset = y * self.cols;
            for x in 0..canvas_cols {
                let cell = self.cells.get(row_offset + x).copied().unwrap_or_default();
                if cur_bold != Some(cell.bold) {
                    cur_bold = Some(cell.bold);
                    b.push_str(if cell.bold { "\x1b[1m" } else { "\x1b[22m" });
                }
                if cur_fg != Some(cell.fg) {
                    cur_fg = Some(cell.fg);
                    let (r, g, bl) = cell.fg;
                    let _ = write!(b, "\x1b[38;2;{r};{g};{bl}m");
                }
                if cur_bg != Some(cell.bg) {
                    cur_bg = Some(cell.bg);
                    let (r, g, bl) = cell.bg;
                    let _ = write!(b, "\x1b[48;2;{r};{g};{bl}m");
                }
                b.push(cell.ch);
            }
            b.push_str("\x1b[0m\x1b[1;36m│\x1b[0m");
        }

        let foot_row = 3 + self.rows;
        let foot = fit_to_width(
            " [←/→] Scene  [P] Term  [F] Fullscreen",
            0,
            canvas_cols,
            false,
        );
        let _ = write!(
            b,
            "\x1b[{foot_row};{col}H\x1b[1;36m│\x1b[0m\x1b[2m{foot}\x1b[0m\x1b[1;36m│\x1b[0m"
        );

        let bot_row = 4 + self.rows;
        let bot_bar: String = std::iter::repeat_n('─', canvas_cols).collect();
        let _ = write!(b, "\x1b[{bot_row};{col}H\x1b[1;36m└{bot_bar}┘\x1b[0m");
    }
}

fn build_mini_saver(model: &TuiModel) -> Ascii {
    let saver_name = model.active_saver_name();
    let tour_mode =
        saver_name.eq_ignore_ascii_case("new") || saver_name.eq_ignore_ascii_case("tour");
    let effect_kind = if tour_mode {
        None
    } else {
        EffectKind::parse(saver_name)
    };
    let custom_opt = (model.mode == ModeChoice::CustomText && !model.custom_text.trim().is_empty())
        .then(|| model.custom_text.clone());
    let is_superfedora = saver_name == "superfedora" && custom_opt.is_none();
    let mut saver = if is_superfedora {
        Ascii::new_superfedora()
            .with_dwell(model.dwell_secs)
            .with_fps(30)
    } else {
        let text_mode = TextMode::parse(model.mode.to_config_str());
        Ascii::new()
            .with_mode(text_mode, custom_opt)
            .with_targets(model.active_targets())
            .with_dwell(model.dwell_secs)
            .with_fps(30)
    };
    if tour_mode {
        saver.enable_tour_mode();
    } else if let Some(effect) = effect_kind {
        saver.pin_effect(effect);
    }
    saver
}
