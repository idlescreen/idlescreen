// SPDX-License-Identifier: MIT

//! Fixed-capacity particle trail history without front-removal shifts.
//!
//! This module tracks historical positions of a particle for rendering tail
//! trails, preserving a fixed maximum window size.

const TRAIL_CAP: usize = 4;

/// Pushes a new position into the particle trail history, removing the oldest
/// element once the history exceeds the fixed `TRAIL_CAP` capacity.
pub fn push_particle_history(history: &mut Vec<(i32, i32)>, x: i32, y: i32) {
    if history.len() < TRAIL_CAP {
        history.push((x, y));
        return;
    }
    history.copy_within(1..TRAIL_CAP, 0);
    history[TRAIL_CAP - 1] = (x, y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_particle_history() {
        let mut history = Vec::new();
        push_particle_history(&mut history, 10, 20);
        assert_eq!(history, vec![(10, 20)]);

        push_particle_history(&mut history, 30, 40);
        push_particle_history(&mut history, 50, 60);
        push_particle_history(&mut history, 70, 80);
        assert_eq!(history, vec![(10, 20), (30, 40), (50, 60), (70, 80)]);

        push_particle_history(&mut history, 90, 100);
        assert_eq!(history, vec![(30, 40), (50, 60), (70, 80), (90, 100)]);
    }
}
