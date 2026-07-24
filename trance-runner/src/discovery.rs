use super::launcher::{ALLOWED_SAVERS, is_allowed_saver};
use std::path::PathBuf;

/// Retrieve directories where screensaver plugins may be installed.
///
/// **Order matters for resolution:** system paths are listed first so that
/// distribution packages under `/usr` win over user-writable trees under
/// `$HOME` / `$XDG_DATA_HOME`. A local overwrite in `~/.local` can still be
/// used when no system plugin exists, but cannot shadow a package-installed
/// `.so` of the same allowlisted name.
pub fn get_screensaver_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. System canonical paths (IdleScreen first; trance legacy still searched)
    dirs.push(PathBuf::from("/usr/libexec/idle/screensavers"));
    dirs.push(PathBuf::from("/usr/local/libexec/idle/screensavers"));
    dirs.push(PathBuf::from("/usr/libexec/idlescreen/screensavers"));
    dirs.push(PathBuf::from("/usr/local/libexec/idlescreen/screensavers"));
    dirs.push(PathBuf::from("/usr/libexec/trance/screensavers"));
    dirs.push(PathBuf::from("/usr/local/libexec/trance/screensavers"));

    // 2. System paths from XDG_DATA_DIRS
    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for part in xdg_data_dirs.split(':') {
        if !part.is_empty() {
            dirs.push(PathBuf::from(part).join("idle").join("screensavers"));
            dirs.push(PathBuf::from(part).join("trance").join("screensavers"));
        }
    }

    // 3. User paths last (optional overrides only when system copy is absent)
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        if !xdg_data.is_empty() {
            dirs.push(PathBuf::from(&xdg_data).join("idle").join("screensavers"));
            dirs.push(PathBuf::from(xdg_data).join("trance").join("screensavers"));
        }
    } else if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        for brand in ["idle", "trance"] {
            dirs.push(
                home_path
                    .join(".local")
                    .join("share")
                    .join(brand)
                    .join("screensavers"),
            );
            dirs.push(
                home_path
                    .join(".local")
                    .join("libexec")
                    .join(brand)
                    .join("screensavers"),
            );
        }
    }

    dirs
}

/// Detects all screensavers by scanning the user and system directories for executables.
/// Automatically falls back to the built-in ALLOWED_SAVERS list.
pub fn detect_screensavers() -> Vec<String> {
    use std::collections::HashSet;

    // Built-in allowlist first for stable ordering / guaranteed presence.
    let mut savers: Vec<String> = ALLOWED_SAVERS.iter().map(|s| (*s).to_string()).collect();
    let mut seen: HashSet<String> = savers.iter().cloned().collect();

    for dir in get_screensaver_dirs() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let is_so = entry.path().extension().is_some_and(|ext| ext == "so");
                let is_exec = metadata.permissions().mode() & 0o111 != 0;
                if !(is_so || is_exec) {
                    continue;
                }
            }
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            if !is_allowed_saver(name) {
                continue;
            }
            // sanitize is infallible when is_allowed_saver returned true
            let clean_name = super::launcher::sanitize_saver_name(name).unwrap_or_default();
            if clean_name.is_empty() {
                continue;
            }
            if seen.insert(clean_name.clone()) {
                savers.push(clean_name);
            }
        }
    }

    savers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_dirs_precede_user_dirs() {
        // With HOME set and no XDG_DATA_HOME, /usr paths must appear before ~/.local.
        let dirs = get_screensaver_dirs();
        let usr = dirs.iter().position(|p| p.starts_with("/usr"));
        let home = dirs
            .iter()
            .position(|p| p.components().any(|c| c.as_os_str() == ".local"));
        assert!(usr.is_some(), "expected at least one /usr plugin dir");
        if let (Some(u), Some(h)) = (usr, home) {
            assert!(
                u < h,
                "system dirs must be searched before user dirs: {dirs:?}"
            );
        }
    }
}
