// SPDX-License-Identifier: Apache-2.0

use super::take_failsafe_arm;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

#[test]
fn take_failsafe_arm_fires_only_once() {
    let armed = AtomicBool::new(true);
    assert!(take_failsafe_arm(&armed));
    assert!(!take_failsafe_arm(&armed));
    assert!(!take_failsafe_arm(&armed));
    assert!(!armed.load(Ordering::SeqCst));
}

#[test]
fn take_failsafe_arm_false_when_disarmed() {
    let armed = AtomicBool::new(false);
    assert!(!take_failsafe_arm(&armed));
    assert!(!armed.load(Ordering::SeqCst));
}

#[test]
fn take_failsafe_arm_can_be_rearmed() {
    let armed = AtomicBool::new(true);
    assert!(take_failsafe_arm(&armed));
    armed.store(true, Ordering::SeqCst);
    assert!(take_failsafe_arm(&armed));
    assert!(!take_failsafe_arm(&armed));
}

#[test]
fn take_failsafe_arm_is_thread_safe_once() {
    let armed = Arc::new(AtomicBool::new(true));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let a = Arc::clone(&armed);
        handles.push(thread::spawn(move || take_failsafe_arm(&a)));
    }
    let wins: usize = handles
        .into_iter()
        .map(|h| h.join().expect("join") as usize)
        .sum();
    assert_eq!(wins, 1, "exactly one thread must take the arm");
    assert!(!armed.load(Ordering::SeqCst));
}
