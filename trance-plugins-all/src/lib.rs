// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 UberMetroid

//! # Trance Plugins All Meta-Crate
//!
//! This is a packaging-only placeholder library for the `trance-plugins-all`
//! Debian package. It does not export any functions or contain active Rust logic.
//!
//! ## Purpose
//!
//! The `trance-plugins-all` package serves as a metapackage in Pop!_OS and Debian.
//! Installing this package automatically installs all individual native screensavers
//! via package dependency management.
//!
//! ## Included Screensavers
//!
//! The following screen effects are installed as part of this suite:
//! - **Beams**: Retro spotlight terminal screensaver.
//! - **Bursts**: Color-bursting expansion effect.
//! - **Chaos**: Chaotic particle attractor.
//! - **Cosmos**: Orbital gravity physics and orbital collapses.
//! - **Glyphs**: Cryptographic/terminal falling character matrix.
//! - **Gnats**: Kinetic swarm simulation.
//! - **Storm**: Raining terminal storm with subtitles.
//!
//! ## Discovery
//!
//! The `trance-runner` automatically discovers screensaver binary libraries located in
//! `/usr/lib/trance/plugins/` (installed by the individual screen packages) and presents
//! them dynamically to the user session applet.
