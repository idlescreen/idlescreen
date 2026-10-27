// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Tests for the scene pace: a scene fills the slot it is given.

use super::super::effects::EffectKind;
use super::super::engine::{EngineSession, SceneStep};
use std::time::Duration;

/// The time one display frame stands for, as the presenter drives it.
const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// Runs a session to its end.
///
/// Reports the display frames it took, the frames the scene actually drew,
/// and the frames it drew, so a test can tell a held frame from a drawn one.
fn run_to_end(session: &mut EngineSession) -> (usize, usize, Vec<String>) {
    let mut displays = 0usize;
    let mut drawn = 0usize;
    let mut seen = Vec::new();
    let mut buffer = String::new();
    loop {
        displays += 1;
        match session.next_frame_into(&mut buffer, FRAME) {
            SceneStep::Advanced => {
                drawn += 1;
                seen.push(buffer.clone());
            }
            SceneStep::Held => {}
            SceneStep::Finished => break,
        }
        if displays > 4000 {
            break;
        }
    }
    (displays, drawn, seen)
}

#[test]
fn a_paced_scene_fills_its_slot_instead_of_racing_through() {
    let slot = 2.6f32;
    let mut session =
        EngineSession::new_paced("IDLESCREEN", EffectKind::Pour, 120, 36, 7, 60, slot)
            .expect("paced session");
    let (displays, drawn, _) = run_to_end(&mut session);
    // The scene is paced over the slot: it plays for the whole of it, and
    // every one of its own frames is drawn along the way.
    let expected = (slot * 60.0) as usize;
    assert!(
        displays.abs_diff(expected) <= 3,
        "the scene should fill its {slot}s slot, it took {displays} frames"
    );
    assert!(
        drawn >= 60,
        "the scene should draw all of its frames, it drew {drawn}"
    );
}

#[test]
fn a_paced_scene_shows_the_same_animation_as_the_arcade_one() {
    let mut paced = EngineSession::new_paced("IDLESCREEN", EffectKind::Mmx, 80, 24, 11, 60, 2.6)
        .expect("paced session");
    let mut arcade =
        EngineSession::new("IDLESCREEN", EffectKind::Mmx, 80, 24, 11, 60).expect("arcade session");
    let (paced_displays, paced_drawn, paced_seen) = run_to_end(&mut paced);
    let (arcade_displays, arcade_drawn, arcade_seen) = run_to_end(&mut arcade);
    // The same animation, step for step, but spread over the whole slot
    // instead of racing through it.
    assert_eq!(paced_drawn, arcade_drawn);
    assert_eq!(paced_seen, arcade_seen);
    assert!(
        paced_displays > arcade_displays,
        "the paced scene should take longer to fill its slot"
    );
}

#[test]
fn a_scene_that_outlasts_its_slot_is_never_spedded_up() {
    let slot = 0.2f32;
    let mut paced =
        EngineSession::new_paced("IDLESCREEN", EffectKind::Blackhole, 120, 36, 3, 60, slot)
            .expect("paced session");
    let mut arcade = EngineSession::new("IDLESCREEN", EffectKind::Blackhole, 120, 36, 3, 60)
        .expect("arcade session");
    let (paced_displays, _, _) = run_to_end(&mut paced);
    let (arcade_displays, _, _) = run_to_end(&mut arcade);
    assert!(
        paced_displays.abs_diff(arcade_displays) <= 1,
        "a scene longer than its slot keeps its own speed"
    );
    assert!(paced_displays > (slot * 60.0) as usize);
}

#[test]
fn a_paced_scene_never_freezes_before_it_ends() {
    for kind in [
        EffectKind::Pour,
        EffectKind::Overflow,
        EffectKind::Blackhole,
        EffectKind::Mmx,
    ] {
        let mut session =
            EngineSession::new_paced("IDLESCREEN", kind, 120, 36, 5, 60, 2.6).expect("session");
        let mut buffer = String::new();
        let mut held = 0usize;
        let mut drawn = 0usize;
        loop {
            match session.next_frame_into(&mut buffer, FRAME) {
                SceneStep::Advanced => {
                    drawn += 1;
                    held = 0;
                }
                SceneStep::Held => {
                    held += 1;
                    assert!(held < 120, "{} held its frame for too long", kind.name());
                }
                SceneStep::Finished => break,
            }
        }
        assert!(drawn > 0, "{} played no frames", kind.name());
    }
}
