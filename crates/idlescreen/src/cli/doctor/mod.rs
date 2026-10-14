// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Diagnostic doctor checking display session, daemon status, configuration, and permissions.

pub mod fix;
pub mod inspect;
#[cfg(test)]
mod tests;
pub mod view;

pub use inspect::{DoctorReport, collect_diagnostics};

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    execute_args(&[])
}

pub fn execute_args(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        view::print_help();
        return Ok(());
    }
    if args.iter().any(|a| a == "--fix") {
        return fix::run_doctor_fix();
    }

    let report = collect_diagnostics();
    if args.iter().any(|a| a == "--json") {
        view::print_json(&report);
    } else if args
        .iter()
        .any(|a| a == "-v" || a == "--verbose" || a == "--all")
    {
        view::print_verbose(&report);
    } else {
        view::print_concise(&report);
    }
    Ok(())
}
