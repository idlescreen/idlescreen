use super::*;
use crate::runner::Screensaver;
use crate::runner::TerminalCell;
use std::time::Duration;

#[test]
fn test_bursts_new() {
    let bursts = Bursts::new();
    assert_eq!(bursts.rockets.len(), 0);
    assert_eq!(bursts.particles.len(), 0);
    assert_eq!(bursts.explosions.len(), 0);
    assert_eq!(bursts.stars.len(), 0);
}

#[test]
fn test_bursts_update_and_draw() {
    let mut bursts = Bursts::new();
    // Prevent slow sys_info calls during tests by setting sys_refresh_timer very negative
    bursts.sys_refresh_timer = -1000.0;

    bursts.update(Duration::from_millis(16), 80, 24);
    let mut grid = vec![TerminalCell::default(); 80 * 24];
    bursts.draw(&mut grid, 80, 24);
    // Completed without panic, at least some initialization should be done
    assert!(!bursts.stars.is_empty());
}

#[test]
fn test_particle_decay() {
    let mut bursts = Bursts::new();
    bursts.sys_refresh_timer = -1000.0;

    // Manually push a particle
    bursts.particles.push(Particle {
        x: 10.0,
        y: 10.0,
        vx: 1.0,
        vy: 1.0,
        color: (255, 255, 255),
        ch: '*',
        life: 0.1,
        max_life: 0.1,
    });

    // Update with delta > 0.1 (150ms)
    bursts.update(Duration::from_millis(150), 80, 24);

    // Particle should be removed because its life (0.1) is less than delta (0.15)
    assert!(
        bursts.particles.is_empty(),
        "Particles must fade away and be removed!"
    );
}

#[test]
fn test_rocket_trajectory() {
    let mut bursts = Bursts::new();
    bursts.sys_refresh_timer = -1000.0;

    // Initialize skyline and size first
    bursts.update(Duration::from_millis(8), 80, 24);
    bursts.on_battery = false;

    // Launch a rocket manually
    bursts.rockets.push(Rocket {
        x: 40.0,
        y: 23.0,
        vx: 5.0,
        vy: -15.0,
        color: (255, 0, 0),
    });

    println!("Test trajectory started...");
    for i in 0..300 {
        bursts.update(Duration::from_millis(8), 80, 24); // ~120Hz physics step
        if !bursts.explosions.is_empty() {
            println!("Exploded at frame {i}");
            return;
        }
    }
    panic!("Rocket did not explode within 300 frames!");
}

#[test]
fn update_clamps_huge_dt_after_resume() {
    // A suspend/resume (or long hitch) can deliver a multi-minute dt.
    // update() must clamp the simulation step (<=0.1s + first-frame init)
    // rather than teleporting particles or exhausting rockets/timers.
    let mut saver = Bursts::new();
    saver.update(std::time::Duration::from_mins(5), 80, 24);
    assert!(
        saver.time_elapsed < 1.0,
        "time_elapsed advanced by {} on a 300s dt — clamp missing",
        saver.time_elapsed
    );
}
