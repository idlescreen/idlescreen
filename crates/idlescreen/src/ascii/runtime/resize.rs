// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Deciding when a changed grid size is worth rebuilding the scene for.

use super::Ascii;

impl Ascii {
    /// Rebuilds the scene when the grid size has changed and has settled.
    ///
    /// A window drag reports a new size on every frame. Rebuilding the scene
    /// for each of them measures a fresh step budget every time, and the
    /// measurement is not cheap: it builds the scene and runs the whole of
    /// it. Waiting for the same size to arrive twice in a row keeps the cost
    /// to one rebuild per drag.
    ///
    /// The rebuild still measures the settled size, so the pace stays exact
    /// for the grid the scene actually ends up on. Until then the scene keeps
    /// playing at the size it was built for, and the draw pass centres it in
    /// the new grid.
    pub fn reinit_when_settled(&mut self, cols: usize, rows: usize) {
        if cols == self.last_cols && rows == self.last_rows {
            self.requested_grid = (cols, rows);
            return;
        }
        // A saver that was never sized takes the first size it is given.
        if self.last_cols == 0 || self.requested_grid == (cols, rows) {
            self.init(cols, rows);
            return;
        }
        self.requested_grid = (cols, rows);
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use crate::ascii::Ascii;
    use crate::ascii::effects::EffectKind;
    use crate::cell::TerminalCell;
    use std::time::Duration;

    const COLS: usize = 80;
    const ROWS: usize = 24;
    const FRAME: Duration = Duration::from_millis(16);

    #[test]
    fn a_resize_is_applied_once_the_size_settles() {
        let mut saver = Ascii::new().with_dwell(5.0);
        saver.init(COLS, ROWS);
        saver.pin_effect(EffectKind::Wipe);

        // A drag: a different size on every frame. The scene must not be
        // rebuilt for each of them, because rebuilding measures a fresh step
        // budget and that costs a whole scene run.
        for c in (COLS + 1)..=(COLS + 5) {
            saver.update(FRAME, c, ROWS);
            assert_eq!(
                saver.last_cols, COLS,
                "the scene was rebuilt for an unsettled size"
            );
        }

        // The same size twice: it has settled, so it is applied.
        let settled = COLS + 5;
        saver.update(FRAME, settled, ROWS);
        assert_eq!(saver.last_cols, settled);
        assert_eq!(saver.frame_grid.len(), settled * ROWS);
    }

    #[test]
    fn a_settled_resize_still_animates() {
        let mut saver = Ascii::new().with_dwell(5.0);
        saver.init(COLS, ROWS);
        saver.pin_effect(EffectKind::Wipe);
        // While the resize settles the scene keeps playing, and the draw
        // pass still produces a frame for the new grid.
        saver.update(FRAME, COLS + 1, ROWS);
        saver.update(FRAME, COLS + 1, ROWS);
        let mut wide = vec![TerminalCell::default(); (COLS + 1) * ROWS];
        saver.draw(&mut wide, COLS + 1, ROWS);
        assert!(wide.iter().any(|c| c.ch != ' '));
    }
}
