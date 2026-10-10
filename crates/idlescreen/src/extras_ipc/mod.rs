// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Host-driven IPC client and supervisor for idlescreen-extras.

pub mod breaker;
pub mod client;
pub mod pidfd;
pub mod protocol;

pub use breaker::{
    CircuitBreaker, is_circuit_breaker_tripped, record_circuit_breaker_failure,
    reset_circuit_breaker,
};
pub use client::{resolve_extras_binary, ExtrasSession};
pub use pidfd::PidFd;
