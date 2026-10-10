use super::*;
use crate::runner::Screensaver;
use std::sync::Mutex;
use std::time::Duration;

/// Process-global env races under parallel tests.
/// Host reads `IDLE_SECONDARY_MONITOR` (see idle-runner sys_info).
static MONITOR_ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_primary_monitor<R>(f: impl FnOnce() -> R) -> R {
    crate::runner::set_thread_secondary_monitor(Some(false));
    let out = f();
    crate::runner::set_thread_secondary_monitor(None);
    out
}

fn with_secondary_monitor<R>(f: impl FnOnce() -> R) -> R {
    crate::runner::set_thread_secondary_monitor(Some(true));
    let out = f();
    crate::runner::set_thread_secondary_monitor(None);
    out
}

#[test]
fn test_aurora_new() {
    let a = Aurora::new();
    assert_eq!(a.curtains.len(), 4); // 3 base + 1 surge
    assert!(a.stars.is_empty()); // seeded on first update/init
    assert_eq!(a.last_cols, 0);
}

#[test]
fn test_aurora_update_and_draw() {
    with_primary_monitor(|| {
        let mut a = Aurora::new();
        a.update(Duration::from_millis(16), 80, 24);
        let mut grid = vec![TerminalCell::default(); 80 * 24];
        a.draw(&mut grid, 80, 24);
        assert_eq!(a.last_cols, 80);
        assert!(!a.stars.is_empty());
        // Every cell must be written — no uninit holes for the host.
        let lit = grid.iter().filter(|c| c.ch != ' ').count();
        assert!(lit > 0, "aurora drew nothing");
    });
}

#[test]
fn test_aurora_secondary_monitor_dims() {
    with_secondary_monitor(|| {
        let mut a = Aurora::new();
        a.update(Duration::from_millis(16), 80, 24);
        let mut grid = vec![TerminalCell::default(); 80 * 24];
        a.draw(&mut grid, 80, 24);
        // No aurora ramp chars on secondary — only stars may be lit.
        let ramp = grid
            .iter()
            .filter(|c| matches!(c.ch, '░' | '▒' | '▓' | '█'))
            .count();
        assert_eq!(ramp, 0, "aurora curtains rendered on secondary monitor");
    });
}

#[test]
fn update_clamps_huge_dt_after_resume() {
    // A suspend/resume (or long hitch) can deliver a multi-minute dt.
    // update() must clamp the simulation step (<=0.1s + first-frame init)
    // rather than lurching curtains or exhausting the surge timer.
    let mut a = Aurora::new();
    a.update(Duration::from_mins(5), 80, 24);
    assert!(
        a.time_elapsed < 1.0,
        "time_elapsed advanced by {} on a 300s dt — clamp missing",
        a.time_elapsed
    );
}

#[test]
fn test_surge_eventually_fires() {
    let mut a = Aurora::new();
    a.surge_opt = true;
    for _ in 0..6000 {
        a.update(Duration::from_millis(16), 80, 24);
        if a.surge_env > 0.5 {
            return;
        }
    }
    panic!("surge never fired within ~96s of simulated time");
}

#[test]
fn test_resize_reseeds_stars() {
    let mut a = Aurora::new();
    a.update(Duration::from_millis(16), 80, 24);
    let n80 = a.stars.len();
    a.update(Duration::from_millis(16), 160, 48);
    assert!(a.stars.len() > n80, "star count must scale with grid area");
}
