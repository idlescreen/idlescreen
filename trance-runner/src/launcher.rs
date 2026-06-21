//! Secure plugin path resolution for screensaver `.so` libraries.

use std::path::{Path, PathBuf};

/// The canonical list of allowed saver basenames.
pub const ALLOWED_SAVERS: &[&str] = &[
    "beams", "bursts", "chaos", "cosmos", "glyphs", "gnats", "storm",
];

/// Controls which directories [`resolve_saver_binary`] may search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaunchMode {
    /// Installed system paths only.
    Daemon,
    /// Installed paths plus local development build trees.
    Preview,
}

/// Reduce a raw name or path to a clean basename, if valid.
pub fn sanitize_saver_name(raw: &str) -> Option<String> {
    let stem = Path::new(raw)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(raw);

    if !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }

    let mut cleaned = stem.to_string();
    if cleaned.starts_with("screensaver-") {
        cleaned = cleaned["screensaver-".len()..].to_string();
    }

    if cleaned.is_empty() {
        return None;
    }

    Some(cleaned)
}

fn dev_plugin_dirs(clean: &str) -> Vec<PathBuf> {
    let Ok(home) = std::env::var("HOME") else {
        return Vec::new();
    };
    let projects = PathBuf::from(home).join("Projects");
    let local76_plugins = projects.join("local76").join("trance-plugins");
    vec![
        local76_plugins.join("target").join("release"),
        local76_plugins.join("target").join("debug"),
        local76_plugins.join(clean).join("target").join("release"),
        local76_plugins.join(clean).join("target").join("debug"),
        projects
            .join(format!("trance-plugin-{clean}"))
            .join("target")
            .join("release"),
        projects
            .join(format!("trance-plugin-{clean}"))
            .join("target")
            .join("debug"),
    ]
}

/// Resolve a saver name to a trusted plugin library path.
pub fn resolve_saver_binary(name: &str, mode: &LaunchMode) -> std::io::Result<PathBuf> {
    let clean = sanitize_saver_name(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown or invalid screensaver name: {name}"),
        )
    })?;

    let candidates = [
        format!("libscreensaver_{clean}.so"),
        format!("lib{clean}.so"),
        clean.clone(),
    ];

    let find_in_dir = |base: &Path| -> Option<PathBuf> {
        for candidate in &candidates {
            let path = base.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
        None
    };

    // Preview prefers local cargo builds over the apt-installed plugin.
    if *mode == LaunchMode::Preview {
        for base in dev_plugin_dirs(&clean) {
            if let Some(path) = find_in_dir(&base) {
                return Ok(path);
            }
        }
    }

    for base in crate::discovery::get_screensaver_dirs() {
        if let Some(path) = find_in_dir(&base) {
            return Ok(path);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("no trusted plugin found for saver '{clean}' (mode {mode:?})"),
    ))
}

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod tests;