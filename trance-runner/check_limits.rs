// SPDX-License-Identifier: MIT

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const MIN_LINES: usize = 25;
const MAX_LINES: usize = 250;
const SRC_DIR: &str = "src";
const REPORT_FILE: &str = "LINE_LIMITS.md";

fn count_lines(path: &Path) -> std::io::Result<usize> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().count())
}

fn visit_dirs(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, files)?;
            } else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src_path = Path::new(SRC_DIR);
    if !src_path.exists() {
        eprintln!("Error: {} directory not found.", SRC_DIR);
        process::exit(1);
    }

    let mut rs_files = Vec::new();
    visit_dirs(src_path, &mut rs_files)?;

    let mut file_stats = Vec::new();
    let mut violations = Vec::new();

    for filepath in rs_files {
        let rel_path = filepath.strip_prefix(".")
            .unwrap_or(&filepath)
            .to_string_lossy()
            .into_owned();
        if let Ok(line_count) = count_lines(&filepath) {
            file_stats.push((rel_path.clone(), line_count));
            if line_count < MIN_LINES || line_count > MAX_LINES {
                violations.push((rel_path, line_count));
            }
        }
    }

    // Sort files by name for consistent markdown output
    file_stats.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate LINE_LIMITS.md
    let mut md_content = String::new();
    md_content.push_str("# Codebase File Line Limits\n\n");
    md_content.push_str(&format!(
        "This project enforces a range of **{} to {} lines** per source file ",
        MIN_LINES, MAX_LINES
    ));
    md_content.push_str("to ensure readability and compatibility with smaller LLMs (like Mistral and Minimax).\n\n");

    md_content.push_str("## Status Report\n\n");
    if !violations.is_empty() {
        md_content.push_str("❌ **WARNING: Some files fall outside the line limit range.**\n\n");
    } else {
        md_content.push_str("✅ **SUCCESS: All files are within limits.**\n\n");
    }

    md_content.push_str("| File | Line Count | Status |\n");
    md_content.push_str("|---|---|---|\n");
    for (path, count) in &file_stats {
        let status = if *count < MIN_LINES {
            format!("❌ Too small (< {})", MIN_LINES)
        } else if *count > MAX_LINES {
            format!("❌ Exceeds limit (> {})", MAX_LINES)
        } else {
            "✅ OK".to_string()
        };
        md_content.push_str(&format!("| [`{}`]({}) | {} | {} |\n", path, path, count, status));
    }

    fs::write(REPORT_FILE, md_content)?;
    println!("Generated {} status report.", REPORT_FILE);

    if !violations.is_empty() {
        println!("\n❌ File Limit Violations Found:");
        for (path, count) in violations {
            if count < MIN_LINES {
                println!("  - {}: {} lines (below {})", path, count, MIN_LINES);
            } else {
                println!("  - {}: {} lines (exceeds {})", path, count, MAX_LINES);
            }
        }
        process::exit(1);
    } else {
        println!("\n✅ All {} files are between {} and {} lines.", file_stats.len(), MIN_LINES, MAX_LINES);
        process::exit(0);
    }
}
