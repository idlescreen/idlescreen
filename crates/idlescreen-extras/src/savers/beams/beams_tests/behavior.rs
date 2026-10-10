use super::super::*;
use super::with_secondary_monitor;
use crate::runner::Screensaver;
use crate::runner::TerminalCell;
use std::time::Duration;

#[test]
fn test_secondary_monitor_rendering() {
    with_secondary_monitor(|| {
        let mut b = Beams::new();
        b.init(80, 24);
        for _ in 0..40 {
            b.update(Duration::from_millis(20), 80, 24);
        }
        let mut grid = vec![TerminalCell::default(); 80 * 24];
        b.draw(&mut grid, 80, 24);

        for (i, cell) in grid.iter().enumerate() {
            assert_eq!(
                cell.bg,
                (0, 0, 0),
                "Cell at index {} has non-black background {:?}",
                i,
                cell.bg
            );
        }

        let drawn_chars = grid.iter().filter(|c| c.ch != ' ' && c.ch != '\0').count();
        assert!(
            drawn_chars > 0,
            "Stars and dust should be drawn on secondary monitor"
        );
    });
}

#[test]
fn test_calm_never_freezes_all_beams() {
    let mut b = Beams::new();
    b.init(80, 24);
    for spot in &mut b.spotlights {
        spot.motion_timer = 0.0;
        spot.is_calm = false;
        spot.motion_blend = 1.0;
    }
    b.update(Duration::from_millis(16), 80, 24);
    let calm = b.spotlights.iter().filter(|s| s.is_calm).count();
    assert!(
        calm <= 2,
        "expected at most 2 calm beams after first wave, got {calm}"
    );
    assert!(
        calm >= 1 || b.spotlights.is_empty(),
        "with free slots, at least one beam should rest when timers fire"
    );

    for _ in 0..500 {
        if b.rng.next_f32() < 0.15 {
            for spot in &mut b.spotlights {
                if !spot.is_calm {
                    spot.motion_timer = 0.0;
                }
            }
        }
        b.update(Duration::from_millis(50), 80, 24);
        let calm = b.spotlights.iter().filter(|s| s.is_calm).count();
        assert!(calm <= 2, "calm cap broken: {calm} beams resting");
        if b.spotlights.len() > 2 {
            assert!(
                calm < b.spotlights.len(),
                "all beams froze at once — looks like a glitch"
            );
        }
    }
}

#[test]
fn update_clamps_huge_dt_after_resume() {
    // A suspend/resume (or long hitch) can deliver a multi-minute dt.
    // update() must clamp the simulation step (<=0.1s + first-frame init)
    // rather than teleporting particles or exhausting rockets/timers.
    let mut saver = Beams::new();
    saver.update(std::time::Duration::from_secs(300), 80, 24);
    assert!(
        saver.time_elapsed < 1.0,
        "time_elapsed advanced by {} on a 300s dt — clamp missing",
        saver.time_elapsed
    );
}
