// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Multi-call binary for IdleScreen mathematical screensaver collection.

pub mod protocol;
pub mod runner;
pub mod savers;
pub mod server;

pub use savers::aurora;
pub use savers::beams;
pub use savers::bursts;
pub use savers::chaos;
pub use savers::cosmos;
pub use savers::glyphs;
pub use savers::gnats;
pub use savers::hearth;
pub use savers::radar;
pub use savers::ripple;
pub use savers::storm;

extern crate self as idle_api;
extern crate self as perf_test_support;
pub use runner::audio;
pub use runner::perf_test_support::*;

use std::os::fd::FromRawFd;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

fn usage() -> ExitCode {
    println!("idlescreen-extras — IdleScreen mathematical screensaver collection helper");
    println!();
    println!("Usage:");
    println!("  idlescreen-extras <saver_name>   Run screensaver via anonymous IPC");
    println!("  idlescreen-extras --list         List available screensavers");
    println!("  idlescreen-extras --help         Show this help message");
    println!();
    println!("Available screensavers:");
    for s in savers::ALL_SAVERS {
        println!("  {s}");
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return usage();
    }

    match args[0].as_str() {
        "--help" | "-h" | "help" => usage(),
        "--version" | "-V" | "version" => {
            println!("idlescreen-extras {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "--list" | "list" => {
            for s in savers::ALL_SAVERS {
                println!("{s}");
            }
            ExitCode::SUCCESS
        }
        saver_name => {
            let fd = if let Ok(s) = std::env::var("IDLE_IPC_FD") {
                s.parse::<i32>().unwrap_or(3)
            } else {
                3
            };

            let socket = unsafe { UnixStream::from_raw_fd(fd) };
            if let Err(e) = server::run(saver_name, socket) {
                eprintln!("idlescreen-extras error: {e}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
    }
}
