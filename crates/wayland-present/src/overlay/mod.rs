// SPDX-License-Identifier: MIT

mod buffer;
mod handlers;
mod state;
mod thread;

pub use thread::{spawn_event_thread, PresenterCommand};