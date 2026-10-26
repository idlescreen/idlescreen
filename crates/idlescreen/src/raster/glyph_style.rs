// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Bitmap glyph style selection for the terminal-cell rasterizer.
//!
//! This is a pure data type: it names the three bitmap treatments the 8x16
//! glyph font can draw. It lives apart from the theme *probe* because the
//! probe reaches for D-Bus and the host's desktop settings, while the style
//! itself is needed by the rasterizer — which must build without any
//! product dependency.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphFontStyle {
    ArcadeBlock,
    CyberSharp,
    RetroRound,
}

impl GlyphFontStyle {
    pub fn parse(raw: &str) -> Self {
        let l = raw.trim().to_ascii_lowercase();
        let sharp = ["sharp", "cyber", "jetbrains", "iosevka", "fira", "hack"];
        let round = ["round", "retro", "ubuntu", "cascadia", "comic"];
        if sharp.iter().any(|k| l.contains(k)) {
            Self::CyberSharp
        } else if round.iter().any(|k| l.contains(k)) {
            Self::RetroRound
        } else {
            Self::ArcadeBlock
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ArcadeBlock => "arcade-block",
            Self::CyberSharp => "cyber-sharp",
            Self::RetroRound => "retro-round",
        }
    }
}
