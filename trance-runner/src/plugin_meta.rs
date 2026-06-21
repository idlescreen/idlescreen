// SPDX-License-Identifier: MIT

/// How a screensaver uses multiple monitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    /// Independent simulation per monitor (default).
    Expand,
    /// One simulation on the primary monitor, letterboxed to all outputs.
    Mirror,
    /// One simulation on the primary monitor; other outputs are blacked out.
    PrimaryOnly,
    /// One simulation spanning the virtual desktop; each monitor shows its region.
    Span,
}

impl DisplayMode {
    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Expand => "expand",
            Self::Mirror => "mirror",
            Self::PrimaryOnly => "primary",
            Self::Span => "span",
        }
    }
}

/// Parse a user-facing display mode string (`primary`, `mirror`, `expand`, `span`).
pub fn parse_display_mode(value: &str) -> Option<DisplayMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "expand" => Some(DisplayMode::Expand),
        "mirror" => Some(DisplayMode::Mirror),
        "primary" | "primary_only" | "primary-only" => Some(DisplayMode::PrimaryOnly),
        "span" => Some(DisplayMode::Span),
        _ => None,
    }
}

/// Effective display mode for a screensaver.
///
/// Priority: `TRANCE_DISPLAY_MODE` env → saver-specific layout default → global config → primary.
/// Cosmos and beams always span unless overridden by env (multi-monitor layout is required).
pub fn display_mode_for(saver_name: &str, configured: Option<DisplayMode>) -> DisplayMode {
    if let Ok(mode) = std::env::var("TRANCE_DISPLAY_MODE") {
        if let Some(parsed) = parse_display_mode(&mode) {
            return parsed;
        }
    }
    if saver_name == "cosmos" || saver_name == "beams" {
        return DisplayMode::Span;
    }
    if let Some(mode) = configured {
        return mode;
    }
    DisplayMode::PrimaryOnly
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_mode_resolution() {
        unsafe {
            std::env::remove_var("TRANCE_DISPLAY_MODE");
        }

        assert_eq!(
            display_mode_for("cosmos", Some(DisplayMode::PrimaryOnly)),
            DisplayMode::Span
        );
        assert_eq!(
            display_mode_for("beams", Some(DisplayMode::PrimaryOnly)),
            DisplayMode::Span
        );
        assert_eq!(
            display_mode_for("storm", Some(DisplayMode::PrimaryOnly)),
            DisplayMode::PrimaryOnly
        );
        assert_eq!(
            display_mode_for("storm", None),
            DisplayMode::PrimaryOnly
        );

        unsafe {
            std::env::set_var("TRANCE_DISPLAY_MODE", "primary");
        }
        assert_eq!(
            display_mode_for("cosmos", Some(DisplayMode::Span)),
            DisplayMode::PrimaryOnly
        );

        unsafe {
            std::env::remove_var("TRANCE_DISPLAY_MODE");
        }
    }
}