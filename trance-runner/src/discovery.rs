use super::launcher::{ALLOWED_SAVERS, is_allowed_saver};
use std::path::PathBuf;

/// Retrieve all trusted directories where screensavers are installed or placed.
/// Supports standard system locations and user-specific directories following XDG specs.
pub fn get_screensaver_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. User path: $XDG_DATA_HOME/trance/screensavers (fallback ~/.local/share/trance/screensavers)
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        if !xdg_data.is_empty() {
            dirs.push(PathBuf::from(xdg_data).join("trance").join("screensavers"));
        }
    } else if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        dirs.push(
            home_path
                .join(".local")
                .join("share")
                .join("trance")
                .join("screensavers"),
        );
        dirs.push(
            home_path
                .join(".local")
                .join("libexec")
                .join("trance")
                .join("screensavers"),
        );
    }

    // 2. System paths from XDG_DATA_DIRS (fallback /usr/local/share, /usr/share)
    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for part in xdg_data_dirs.split(':') {
        if !part.is_empty() {
            dirs.push(PathBuf::from(part).join("trance").join("screensavers"));
        }
    }

    // 3. System canonical/historical paths
    dirs.push(PathBuf::from("/usr/libexec/trance/screensavers"));
    dirs.push(PathBuf::from("/usr/local/libexec/trance/screensavers"));

    dirs
}

/// Detects all screensavers by scanning the user and system directories for executables.
/// Automatically falls back to the built-in ALLOWED_SAVERS list.
pub fn detect_screensavers() -> Vec<String> {
    let mut savers = Vec::new();

    // Always start with built-in allowed list for fallback / guaranteed list
    for s in ALLOWED_SAVERS {
        savers.push(s.to_string());
    }

    let dirs = get_screensaver_dirs();
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata()
                    && metadata.is_file()
                {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        // Check if file is executable
                        if metadata.permissions().mode() & 0o111 != 0
                            && let Some(name) = entry.file_name().to_str()
                            && is_allowed_saver(name)
                        {
                            let clean_name = super::launcher::sanitize_saver_name(name).unwrap();
                            if !savers.contains(&clean_name) {
                                savers.push(clean_name);
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        if let Some(name) = entry.file_name().to_str() {
                            if is_allowed_saver(name) {
                                let clean_name =
                                    super::launcher::sanitize_saver_name(name).unwrap();
                                if !savers.contains(&clean_name) {
                                    savers.push(clean_name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    savers
}
