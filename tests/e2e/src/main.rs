// SPDX-License-Identifier: Apache-2.0
//! IdleScreen E2E Test Suite Runner Binary.

use std::process::Command;

fn main() {
    println!("============================================================");
    println!("   IdleScreen End-to-End Test Suite (4-Tier Architecture)   ");
    println!("============================================================");

    let status = Command::new("cargo")
        .args(["test", "--manifest-path", "tests/e2e/Cargo.toml"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("All E2E test suites passed successfully.");
            std::process::exit(0);
        }
        Ok(s) => {
            eprintln!("E2E test suite failed with exit code: {:?}", s.code());
            std::process::exit(s.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Failed to invoke cargo test: {e}");
            std::process::exit(1);
        }
    }
}
