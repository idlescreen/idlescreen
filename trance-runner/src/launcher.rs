//! Secure plugin path resolution for screensaver `.so` libraries.

use std::path::{Path, PathBuf};

/// Errors that can occur during plugin loading and initialization.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin name '{0}' is not in the allowlist")]
    NotAllowed(String),
    #[error("plugin path contains '..' (path traversal attempt)")]
    PathTraversal,
    #[error("invalid plugin name: {0}")]
    InvalidName(String),
    #[error("failed to load library: {0}")]
    LoadFailure(#[from] libloading::Error),
    #[error("symbol '{0}' not found in plugin")]
    SymbolMissing(&'static str),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

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

/// Whether `name` resolves to a built-in screensaver package.
pub fn is_allowed_saver(name: &str) -> bool {
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    sanitize_saver_name(name)
        .as_deref()
        .is_some_and(|clean| ALLOWED_SAVERS.contains(&clean))
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
    let ubermetroid_plugins = projects.join("ubermetroid").join("trance-plugins");
    vec![
        ubermetroid_plugins.join("target").join("release"),
        ubermetroid_plugins.join("target").join("debug"),
        ubermetroid_plugins
            .join(clean)
            .join("target")
            .join("release"),
        ubermetroid_plugins.join(clean).join("target").join("debug"),
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

/// True when `path` resolves under one of the trusted plugin directories.
pub fn is_trusted_plugin_path(path: &Path, trusted_dirs: &[PathBuf]) -> bool {
    let canonical = match std::fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return false,
    };
    trusted_dirs.iter().any(|dir| {
        std::fs::canonicalize(dir)
            .ok()
            .is_some_and(|canonical_dir| canonical.starts_with(&canonical_dir))
    })
}

fn trusted_plugin_dirs(clean: &str, mode: &LaunchMode) -> Vec<PathBuf> {
    let mut dirs = crate::discovery::get_screensaver_dirs();
    if *mode == LaunchMode::Preview {
        dirs.extend(dev_plugin_dirs(clean));
    }
    dirs
}

/// Resolve a saver name to a trusted plugin library path.
pub fn resolve_saver_binary(name: &str, mode: &LaunchMode) -> std::io::Result<PathBuf> {
    if name.contains('/') || name.contains('\\') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("saver name must not be a path: {name}"),
        ));
    }
    let clean = sanitize_saver_name(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown or invalid screensaver name: {name}"),
        )
    })?;

    if !ALLOWED_SAVERS.contains(&clean.as_str()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("screensaver '{clean}' is not in the trusted allowlist"),
        ));
    }

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

    let trusted_dirs = trusted_plugin_dirs(&clean, mode);
    let dev_dirs = dev_plugin_dirs(&clean);
    let search_order: Vec<&Path> = if *mode == LaunchMode::Preview {
        dev_dirs
            .iter()
            .map(|p| p.as_path())
            .chain(trusted_dirs.iter().map(|p| p.as_path()))
            .collect()
    } else {
        trusted_dirs.iter().map(|p| p.as_path()).collect()
    };

    for base in search_order {
        if let Some(path) = find_in_dir(base)
            && is_trusted_plugin_path(&path, &trusted_dirs)
        {
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
