use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorCellBounds {
    pub start_col: usize,
    pub end_col: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub is_primary: bool,
}

impl MonitorCellBounds {
    pub fn width(&self) -> usize {
        self.end_col.saturating_sub(self.start_col)
    }

    pub fn height(&self) -> usize {
        self.end_row.saturating_sub(self.start_row)
    }

    pub fn center_col(&self) -> usize {
        self.start_col + self.width() / 2
    }

    pub fn center_row(&self) -> usize {
        self.start_row + self.height() / 2
    }

    /// Returns true if `(col, row)` is inside this bounds (half-open: `end_col`/`end_row` excluded).
    ///
    /// # Example
    ///
    /// ```
    /// use trance_api::MonitorCellBounds;
    /// let b = MonitorCellBounds {
    ///     start_col: 0,
    ///     end_col: 10,
    ///     start_row: 0,
    ///     end_row: 5,
    ///     is_primary: true,
    /// };
    /// assert!(b.contains(5, 3));
    /// assert!(!b.contains(11, 3));
    /// assert!(!b.contains(10, 0)); // end_col is exclusive
    /// ```
    pub fn contains(&self, col: usize, row: usize) -> bool {
        col >= self.start_col && col < self.end_col && row >= self.start_row && row < self.end_row
    }
}

pub static MONITOR_BOUNDS_CALLBACK: OnceLock<fn(usize, usize) -> MonitorCellBounds> =
    OnceLock::new();
pub static IS_SECONDARY_MONITOR_CALLBACK: OnceLock<fn() -> bool> = OnceLock::new();

pub fn get_primary_monitor_bounds(cols: usize, rows: usize) -> MonitorCellBounds {
    if let Some(callback) = MONITOR_BOUNDS_CALLBACK.get() {
        return callback(cols, rows);
    }
    if let Some(bounds) = cached_primary_bounds_from_env() {
        return bounds;
    }
    MonitorCellBounds {
        start_col: 0,
        end_col: cols,
        start_row: 0,
        end_row: rows,
        is_primary: true,
    }
}

static ENV_PRIMARY_BOUNDS: OnceLock<Mutex<Option<MonitorCellBounds>>> = OnceLock::new();

fn env_bounds_cache() -> &'static Mutex<Option<MonitorCellBounds>> {
    ENV_PRIMARY_BOUNDS.get_or_init(|| Mutex::new(None))
}

fn cached_primary_bounds_from_env() -> Option<MonitorCellBounds> {
    let mut cache = env_bounds_cache().lock().unwrap();
    if cache.is_none() {
        *cache = read_primary_bounds_from_env();
    }
    *cache
}

fn read_primary_bounds_from_env() -> Option<MonitorCellBounds> {
    let start_col = std::env::var("TRANCE_PRIMARY_START_COL")
        .ok()?
        .parse()
        .ok()?;
    let end_col = std::env::var("TRANCE_PRIMARY_END_COL").ok()?.parse().ok()?;
    let start_row = std::env::var("TRANCE_PRIMARY_START_ROW")
        .ok()?
        .parse()
        .ok()?;
    let end_row = std::env::var("TRANCE_PRIMARY_END_ROW").ok()?.parse().ok()?;
    if end_col <= start_col || end_row <= start_row {
        return None;
    }
    const MAX_GRID: usize = 16_384;
    if end_col > MAX_GRID || end_row > MAX_GRID {
        return None;
    }
    Some(MonitorCellBounds {
        start_col,
        end_col,
        start_row,
        end_row,
        is_primary: true,
    })
}

pub fn publish_primary_bounds(bounds: MonitorCellBounds) {
    // SAFETY (Phase 4 note): the `unsafe std::env::set_var` calls below are a known
    // hazard — `std::env::set_var` is not thread-safe and the surrounding `unsafe`
    // blocks provide no actual safety guarantee. A follow-up Phase 4 agent working
    // on the daemon crate will replace this IPC mechanism with a thread-safe channel,
    // at which point these `unsafe` blocks and the env-var fallback in
    // `read_primary_bounds_from_env` can be removed entirely. Do not remove them yet.
    unsafe {
        std::env::set_var("TRANCE_PRIMARY_START_COL", bounds.start_col.to_string());
        std::env::set_var("TRANCE_PRIMARY_END_COL", bounds.end_col.to_string());
        std::env::set_var("TRANCE_PRIMARY_START_ROW", bounds.start_row.to_string());
        std::env::set_var("TRANCE_PRIMARY_END_ROW", bounds.end_row.to_string());
    }
    *env_bounds_cache().lock().unwrap() = Some(bounds);
}

pub fn clear_primary_bounds() {
    // See `publish_primary_bounds` for the Phase 4 hazard note.
    unsafe {
        std::env::remove_var("TRANCE_PRIMARY_START_COL");
        std::env::remove_var("TRANCE_PRIMARY_END_COL");
        std::env::remove_var("TRANCE_PRIMARY_START_ROW");
        std::env::remove_var("TRANCE_PRIMARY_END_ROW");
    }
    *env_bounds_cache().lock().unwrap() = None;
}

pub fn is_secondary_monitor() -> bool {
    if let Some(callback) = IS_SECONDARY_MONITOR_CALLBACK.get() {
        callback()
    } else {
        std::env::var("TRANCE_SECONDARY_MONITOR").is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(
        start_col: usize,
        end_col: usize,
        start_row: usize,
        end_row: usize,
    ) -> MonitorCellBounds {
        MonitorCellBounds {
            start_col,
            end_col,
            start_row,
            end_row,
            is_primary: true,
        }
    }

    #[test]
    fn bounds_contains_inside() {
        let b = bounds(0, 10, 0, 5);
        assert!(b.contains(5, 3));
    }

    #[test]
    fn bounds_excludes_outside() {
        let b = bounds(0, 10, 0, 5);
        assert!(!b.contains(11, 3));
        assert!(!b.contains(5, 6));
    }

    #[test]
    fn bounds_excludes_end_exclusive() {
        let b = bounds(0, 10, 0, 5);
        assert!(!b.contains(10, 0));
        assert!(!b.contains(0, 5));
    }

    #[test]
    fn bounds_width_height() {
        let b = bounds(0, 10, 0, 5);
        assert_eq!(b.width(), 10);
        assert_eq!(b.height(), 5);
    }

    #[test]
    fn bounds_width_height_saturate_when_inverted() {
        let b = bounds(8, 4, 6, 2);
        assert_eq!(b.width(), 0);
        assert_eq!(b.height(), 0);
    }

    #[test]
    fn bounds_centers() {
        let b = bounds(0, 10, 0, 6);
        assert_eq!(b.center_col(), 5);
        assert_eq!(b.center_row(), 3);
    }

    #[test]
    fn get_primary_monitor_bounds_default_is_full_grid() {
        // No callback set and no env vars in test by default
        unsafe {
            std::env::remove_var("TRANCE_PRIMARY_START_COL");
            std::env::remove_var("TRANCE_PRIMARY_END_COL");
            std::env::remove_var("TRANCE_PRIMARY_START_ROW");
            std::env::remove_var("TRANCE_PRIMARY_END_ROW");
        }
        clear_primary_bounds();
        let b = get_primary_monitor_bounds(80, 24);
        assert_eq!(b.start_col, 0);
        assert_eq!(b.end_col, 80);
        assert_eq!(b.start_row, 0);
        assert_eq!(b.end_row, 24);
        assert!(b.is_primary);
    }

    #[test]
    fn publish_then_get_primary_bounds_round_trip() {
        clear_primary_bounds();
        publish_primary_bounds(bounds(2, 8, 1, 4));
        let b = get_primary_monitor_bounds(80, 24);
        assert_eq!(b.start_col, 2);
        assert_eq!(b.end_col, 8);
        assert_eq!(b.start_row, 1);
        assert_eq!(b.end_row, 4);
        clear_primary_bounds();
    }

    #[test]
    fn is_secondary_monitor_default_false() {
        unsafe {
            std::env::remove_var("TRANCE_SECONDARY_MONITOR");
        }
        assert!(!is_secondary_monitor());
    }
}
