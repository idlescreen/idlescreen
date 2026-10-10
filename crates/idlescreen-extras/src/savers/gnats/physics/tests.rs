use super::*;
use crate::gnats::{Attractor, Firefly, KillSpark, Star};
use crate::runner::LcgRng;

#[test]
fn test_update_attractors() {
    let mut attractors = vec![
        Attractor {
            x: 0.0,
            y: 0.0,
            color: (255, 255, 255),
            phase: 0.0,
            speed: 1.0,
        },
        Attractor {
            x: 0.0,
            y: 0.0,
            color: (255, 255, 255),
            phase: 1.0,
            speed: 1.0,
        },
        Attractor {
            x: 0.0,
            y: 0.0,
            color: (255, 255, 255),
            phase: 2.0,
            speed: 1.0,
        },
    ];
    update_attractors(&mut attractors, 1.0, 80.0, 24.0);
    // Positions should have changed from 0.0
    assert!(attractors[0].x != 0.0);
    assert!(attractors[0].y != 0.0);
}

#[test]
fn test_decay_logo_excitations() {
    let mut excitations = vec![1.0, 0.5, 0.0];
    decay_logo_excitations(&mut excitations, 0.1);
    // With 0.1 delta and decay rate of 1.8, excitation decreases by 0.18
    assert!((excitations[0] - 0.82).abs() < 1e-5);
    assert!((excitations[1] - 0.32).abs() < 1e-5);
    assert_eq!(excitations[2], 0.0);
}

#[test]
fn test_update_kill_sparks() {
    let mut sparks = vec![KillSpark {
        x: 10.0,
        y: 10.0,
        vx: 2.0,
        vy: -1.0,
        color: (255, 0, 0),
        life: 1.0,
    }];
    update_kill_sparks(&mut sparks, 0.1);
    assert!((sparks[0].x - 10.2).abs() < 1e-5);
    assert!((sparks[0].y - 9.9).abs() < 1e-5);
    assert!((sparks[0].life - 0.8).abs() < 1e-5);

    // Test retention/removal when life <= 0
    update_kill_sparks(&mut sparks, 0.5);
    assert_eq!(sparks.len(), 0);
}

#[test]
fn test_update_stars() {
    let mut stars = vec![Star {
        x: 0.5,
        y: 0.5,
        phase: 0.0,
        ch: '.',
        excitation: 0.0,
    }];
    // A firefly close to the star (cols_f=80, rows_f=24, so star is at (40.0, 12.0))
    let fireflies = vec![Firefly {
        x: 40.5,
        y: 12.0,
        vx: 0.0,
        vy: 0.0,
        color: (0, 0, 0),
        size: 1,
        speed_mult: 1.0,
        history: vec![],
        blink_phase: 0.0,
        blink_rate: 2.0,
    }];
    update_stars(&mut stars, &fireflies, 0.1, 80.0, 24.0);
    assert!(stars[0].excitation > 0.0);

    // Decay stars test
    let mut star2 = vec![Star {
        x: 0.5,
        y: 0.5,
        phase: 0.0,
        ch: '.',
        excitation: 1.0,
    }];
    update_stars(&mut star2, &[], 0.1, 80.0, 24.0);
    assert!((star2[0].excitation - 0.88).abs() < 1e-5);
}

#[test]
fn test_predator_prey_forces() {
    let mut fireflies = vec![
        Firefly {
            x: 10.0,
            y: 10.0,
            vx: 0.0,
            vy: 0.0,
            color: (0, 0, 0),
            size: 3,
            speed_mult: 1.0,
            history: vec![],
            blink_phase: 0.0,
            blink_rate: 2.0,
        }, // Predator (large)
        Firefly {
            x: 10.5,
            y: 10.5,
            vx: 0.0,
            vy: 0.0,
            color: (0, 0, 0),
            size: 1,
            speed_mult: 1.0,
            history: vec![],
            blink_phase: 1.0,
            blink_rate: 2.5,
        }, // Prey (small)
    ];
    let attractors = vec![];
    let mut rng = LcgRng::new(42);
    let dead = compute_firefly_forces_and_update(
        &mut fireflies,
        &attractors,
        0.0,
        80.0,
        24.0,
        &mut rng,
        0.01,
        1.0,
        true, // predator window active
    );

    // Prey should run away (flee force), predator should chase, and since distance is very small (< 1.1), prey is marked dead
    assert!(dead.contains(&1));
}
