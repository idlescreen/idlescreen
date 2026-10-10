// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! CLI argument parser and options model for the preview subcommand.

use std::time::Duration;

/// Parsed options for `idlescreen preview`.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewOptions {
    pub saver: Option<String>,
    pub timeout: Option<Duration>,
    pub mode: Option<String>,
    pub dwell: Option<f32>,
    pub text: Option<String>,
}

/// Parses CLI arguments for the preview subcommand.
pub fn parse_preview_args(
    args: &[String],
) -> Result<Option<PreviewOptions>, Box<dyn std::error::Error>> {
    let mut saver = None;
    let mut timeout = None;
    let mut mode = None;
    let mut dwell = None;
    let mut text = None;
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_preview_help();
                return Ok(None);
            }
            "-t" | "--timeout" => {
                let val = iter
                    .next()
                    .ok_or("preview: --timeout requires duration in seconds")?;
                let secs: f64 = val
                    .parse()
                    .map_err(|_| "preview: invalid duration for --timeout")?;
                timeout = Some(Duration::from_secs_f64(secs.max(0.0)));
            }
            s if s.starts_with("--timeout=") => {
                let val = &s["--timeout=".len()..];
                let secs: f64 = val
                    .parse()
                    .map_err(|_| "preview: invalid duration for --timeout")?;
                timeout = Some(Duration::from_secs_f64(secs.max(0.0)));
            }
            "-d" | "--dwell" => {
                let val = iter
                    .next()
                    .ok_or("preview: --dwell requires duration in seconds")?;
                let secs: f32 = val
                    .parse()
                    .map_err(|_| "preview: invalid duration for --dwell")?;
                dwell = Some(secs.max(0.1));
            }
            s if s.starts_with("--dwell=") => {
                let val = &s["--dwell=".len()..];
                let secs: f32 = val
                    .parse()
                    .map_err(|_| "preview: invalid duration for --dwell")?;
                dwell = Some(secs.max(0.1));
            }
            "-m" | "--mode" => {
                let val = iter
                    .next()
                    .ok_or("preview: --mode requires mode (cycle, os, de, kernel, custom)")?;
                mode = Some(val.clone());
            }
            s if s.starts_with("--mode=") => {
                let val = &s["--mode=".len()..];
                mode = Some(val.to_string());
            }
            "--text" => {
                let val = iter.next().ok_or("preview: --text requires text string")?;
                text = Some(val.clone());
            }
            s if s.starts_with("--text=") => {
                let val = &s["--text=".len()..];
                text = Some(val.to_string());
            }
            s if !s.starts_with('-') => {
                if saver.is_none() {
                    saver = Some(s.to_string());
                }
            }
            other => {
                return Err(format!("preview: unknown option '{other}'").into());
            }
        }
    }

    Ok(Some(PreviewOptions {
        saver,
        timeout,
        mode,
        dwell,
        text,
    }))
}

fn print_preview_help() {
    println!("idlescreen preview — Preview screensavers in terminal");
    println!();
    println!("Usage:");
    println!("  idlescreen preview [saver] [options]");
    println!();
    println!("Options:");
    println!("  -t, --timeout <secs>   Automatically exit after <secs> seconds");
    println!("  -d, --dwell <secs>     Dwell duration in seconds per effect/mutation");
    println!("  -m, --mode <mode>      Text mode: cycle (default), os, de, kernel, custom");
    println!("      --text <string>    Set custom text banner");
    println!("  -h, --help             Show this help message");
    println!();
    println!("Examples:");
    println!("  idlescreen preview                 # Preview active screensaver");
    println!("  idlescreen preview --dwell 2       # Rotate text/effects every 2s");
    println!("  idlescreen preview --mode kernel   # Preview kernel version text");
    println!("  idlescreen preview storm           # Preview 'storm' screensaver");
    println!("  idlescreen preview --timeout 5     # Run preview for 5 seconds");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_preview_args_empty() {
        let args: Vec<String> = vec![];
        let opts = parse_preview_args(&args).unwrap().unwrap();
        assert_eq!(opts.saver, None);
        assert_eq!(opts.timeout, None);
        assert_eq!(opts.mode, None);
        assert_eq!(opts.dwell, None);
        assert_eq!(opts.text, None);
    }

    #[test]
    fn test_parse_preview_args_saver_only() {
        let args = vec!["storm".to_string()];
        let opts = parse_preview_args(&args).unwrap().unwrap();
        assert_eq!(opts.saver.as_deref(), Some("storm"));
    }

    #[test]
    fn test_parse_preview_args_all_flags() {
        let args = vec![
            "--timeout".to_string(),
            "3.5".to_string(),
            "--dwell".to_string(),
            "2.0".to_string(),
            "--mode".to_string(),
            "kernel".to_string(),
            "--text".to_string(),
            "hello".to_string(),
        ];
        let opts = parse_preview_args(&args).unwrap().unwrap();
        assert_eq!(opts.timeout, Some(Duration::from_millis(3500)));
        assert_eq!(opts.dwell, Some(2.0));
        assert_eq!(opts.mode.as_deref(), Some("kernel"));
        assert_eq!(opts.text.as_deref(), Some("hello"));
    }

    #[test]
    fn test_parse_preview_help_returns_none() {
        let args = vec!["--help".to_string()];
        assert!(parse_preview_args(&args).unwrap().is_none());
    }
}
