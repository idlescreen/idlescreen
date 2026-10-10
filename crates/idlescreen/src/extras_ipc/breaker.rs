// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! 60 Hz crash-loop fork-bomb prevention circuit breaker.

use std::time::Instant;

pub const CIRCUIT_BREAKER_MAX_FAILURES: usize = 3;
pub const CIRCUIT_BREAKER_WINDOW_SECS: u64 = 60;

pub struct CircuitBreaker {
    failures: Vec<Instant>,
    tripped: bool,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            failures: Vec::with_capacity(CIRCUIT_BREAKER_MAX_FAILURES + 1),
            tripped: false,
        }
    }

    pub fn is_tripped(&self) -> bool {
        self.tripped
    }

    /// Records an abnormal exit or timeout.
    /// Returns true if this failure caused the circuit breaker to trip.
    pub fn record_failure(&mut self) -> bool {
        if self.tripped {
            return false;
        }

        let now = Instant::now();
        self.failures
            .retain(|&t| now.duration_since(t).as_secs() < CIRCUIT_BREAKER_WINDOW_SECS);
        self.failures.push(now);

        if self.failures.len() >= CIRCUIT_BREAKER_MAX_FAILURES {
            self.tripped = true;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.failures.clear();
        self.tripped = false;
    }
}

static GLOBAL_CIRCUIT_BREAKER: std::sync::Mutex<Option<CircuitBreaker>> =
    std::sync::Mutex::new(None);

pub fn is_circuit_breaker_tripped() -> bool {
    if let Ok(guard) = GLOBAL_CIRCUIT_BREAKER.lock() {
        guard.as_ref().is_some_and(CircuitBreaker::is_tripped)
    } else {
        false
    }
}

pub fn record_circuit_breaker_failure() -> bool {
    if let Ok(mut guard) = GLOBAL_CIRCUIT_BREAKER.lock() {
        let cb = guard.get_or_insert_with(CircuitBreaker::new);
        cb.record_failure()
    } else {
        false
    }
}

pub fn reset_circuit_breaker() {
    if let Ok(mut guard) = GLOBAL_CIRCUIT_BREAKER.lock()
        && let Some(ref mut cb) = *guard
    {
        cb.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_tripping() {
        let mut cb = CircuitBreaker::new();
        assert!(!cb.is_tripped());
        assert!(!cb.record_failure());
        assert!(!cb.is_tripped());
        assert!(!cb.record_failure());
        assert!(!cb.is_tripped());
        assert!(cb.record_failure()); // 3rd failure trips
        assert!(cb.is_tripped());
        assert!(!cb.record_failure()); // already tripped
    }
}
