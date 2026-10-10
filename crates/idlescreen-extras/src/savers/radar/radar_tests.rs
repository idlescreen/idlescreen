// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn test_radar_new() {
    let radar = Radar::new();
    assert_eq!(radar.sweep_angle, 0.0);
    assert!(radar.enemies.is_empty());
    assert!(radar.defenders.is_empty());
    assert!(radar.jets.is_empty());
    assert!(radar.lasers.is_empty());
    assert!(radar.particles.is_empty());
}

#[test]
fn test_radar_init_clears() {
    let mut radar = Radar::new();
    radar.sweep_angle = 1.5;
    radar.enemies.push(EnemyShip {
        id: 1,
        x: 10.0,
        y: 10.0,
        vx: 1.0,
        vy: 1.0,
        visibility: 3.0,
        is_heavy: true,
        target_type: 1,
        health: 1,
        altitude: 0.0,
        active: true,
    });
    radar.jets.push(RadarJet {
        x: 10.0,
        y: 10.0,
        heading_ch: '▶',
        sweep_angle: 0.0,
        t: 0.0,
        lock_target_id: None,
        lock_timer: 0.0,
        altitude: 1.5,
        active: true,
    });
    radar.lasers.push(LaserBeam {
        from_x: 0.0,
        from_y: 0.0,
        to_x: 10.0,
        to_y: 10.0,
        life: 0.4,
        max_life: 0.4,
    });
    radar.init(80, 24);
    assert_eq!(radar.sweep_angle, 0.0);
    assert!(radar.enemies.is_empty());
    assert!(radar.jets.is_empty());
    assert!(radar.lasers.is_empty());
}

#[test]
fn test_radar_update_increments_sweep() {
    let mut radar = Radar::new();
    let old_sweep = radar.sweep_angle;
    radar.update(Duration::from_millis(100), 80, 24);
    assert!(radar.sweep_angle > old_sweep);
}

#[test]
fn test_radar_enemy_detection() {
    let mut radar = Radar::new();
    // Place an enemy at angle ~0
    radar.enemies.push(EnemyShip {
        id: 1,
        x: 60.0, // to the right of center (40, 12)
        y: 12.0,
        vx: 0.0,
        vy: 0.0,
        visibility: 0.0,
        is_heavy: true,
        target_type: 1,
        health: 1,
        altitude: 0.0,
        active: true,
    });
    // Set sweep angle close to 0
    radar.sweep_angle = 0.02;
    radar.update(Duration::from_millis(10), 80, 24);
    assert!(radar.enemies.first().unwrap().visibility > 0.0);
    assert!(!radar.defenders.is_empty());
}

#[test]
fn update_clamps_huge_dt_after_resume() {
    // A suspend/resume (or long hitch) can deliver a multi-minute dt.
    // update() must clamp the simulation step (<=0.1s + first-frame init)
    // rather than teleporting particles or exhausting rockets/timers.
    let mut saver = Radar::new();
    saver.update(std::time::Duration::from_secs(300), 80, 24);
    assert!(
        saver.time_elapsed < 1.0,
        "time_elapsed advanced by {} on a 300s dt — clamp missing",
        saver.time_elapsed
    );
}
