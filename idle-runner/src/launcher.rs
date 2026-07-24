//! Secure plugin path resolution for screensaver `.so` libraries.

use std::path::{Path, PathBuf};

use crate::launcher_resolve::{cleaned_allowed_name, search_trusted_plugin};

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
    #[error("plugin API version {found} incompatible with host {expected}")]
    ApiVersionMismatch { found: u32, expected: u32 },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// The canonical list of allowed saver basenames.
pub const ALLOWED_SAVERS: &[&str] = &[
    "beams", "bursts", "chaos", "cosmos", "glyphs", "gnats", "radar", "storm", "hearth", "ripple",
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
    let mut stem = Path::new(raw)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(raw)
        .to_string();

    if stem.starts_with("libscreensaver_") {
        stem = stem["libscreensaver_".len()..].to_string();
    } else if stem.starts_with("lib") {
        stem = stem["lib".len()..].to_string();
    }

    if stem.starts_with("screensaver-") {
        stem = stem["screensaver-".len()..].to_string();
    }
    // Package name form: idle-saver-beams → beams
    if stem.starts_with("idle-saver-") {
        stem = stem["idle-saver-".len()..].to_string();
    }

    if !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }

    if stem.is_empty() {
        return None;
    }

    Some(stem)
}

pub use crate::launcher_trust::is_trusted_plugin_path;

/// Resolve a saver name to a trusted plugin library path.
pub fn resolve_saver_binary(name: &str, mode: &LaunchMode) -> std::io::Result<PathBuf> {
    let clean = cleaned_allowed_name(name)?;
    search_trusted_plugin(&clean, mode)
}

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "launcher_proptest.rs"]
mod proptests;
