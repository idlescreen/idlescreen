// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! System theme adaptation probe (accent color, light/dark color-scheme, and bitmap glyph style).

use crate::raster::glyph_style::GlyphFontStyle;
// The desktop probes below are the only thing that reads the host's theme
// settings, and they are the only thing that needs a filesystem path. The
// pure parsers stay available to the test suite in either build.
#[cfg(any(feature = "product", test))]
use std::fs;
#[cfg(any(feature = "product", test))]
use std::path::Path;
#[cfg(feature = "product")]
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeColorScheme {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemTheme {
    pub accent_rgb: (u8, u8, u8),
    pub scheme: ThemeColorScheme,
    pub font_style: GlyphFontStyle,
    pub bg_rgb: (u8, u8, u8),
}

impl Default for SystemTheme {
    fn default() -> Self {
        Self::from_parts(
            (81, 162, 218),
            ThemeColorScheme::Dark,
            GlyphFontStyle::ArcadeBlock,
        )
    }
}

impl SystemTheme {
    #[rustfmt::skip]
    pub fn from_parts(accent_rgb: (u8, u8, u8), scheme: ThemeColorScheme, font_style: GlyphFontStyle) -> Self {
        let bg_rgb = if scheme == ThemeColorScheme::Light { (10, 18, 34) } else { (0, 0, 0) };
        Self { accent_rgb, scheme, font_style, bg_rgb }
    }
}

/// Detects the active desktop theme with zero steady-state overhead (`OnceLock` cached).
#[inline]
pub fn detect_system_theme() -> SystemTheme {
    *CACHED_THEME.get_or_init(|| {
        let base = probe_system_theme_uncached();
        resolve_system_theme_with(base, |k| std::env::var(k).ok())
    })
}

static CACHED_THEME: OnceLock<SystemTheme> = OnceLock::new();

/// Resolves theme overrides using a caller-supplied lookup function (used at startup and in tests).
pub fn resolve_system_theme_with<F>(mut theme: SystemTheme, get_var: F) -> SystemTheme
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(v) = get_var("IDLE_THEME_ACCENT")
        && let Some(rgb) = parse_color_str(&v)
    {
        theme.accent_rgb = rgb;
    }
    if let Some(v) = get_var("IDLE_THEME_MODE").or_else(|| get_var("GTK_THEME")) {
        let l = v.trim().to_ascii_lowercase();
        if l.contains("light") || l == "2" {
            theme.scheme = ThemeColorScheme::Light;
        } else if l.contains("dark") || l == "1" {
            theme.scheme = ThemeColorScheme::Dark;
        }
    }
    if let Some(v) = get_var("IDLE_FONT_STYLE").or_else(|| get_var("IDLE_FONT")) {
        theme.font_style = GlyphFontStyle::parse(&v);
    }
    SystemTheme::from_parts(theme.accent_rgb, theme.scheme, theme.font_style)
}

fn probe_system_theme_uncached() -> SystemTheme {
    // With the product feature there is a live desktop session to ask.
    #[cfg(feature = "product")]
    {
        let (mut accent, mut scheme, mut font) = (None, None, None);
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let cfg = std::env::var_os("XDG_CONFIG_HOME")
                .map_or_else(|| home.join(".config"), PathBuf::from);
            probe_config_files(&cfg, &mut accent, &mut scheme, &mut font);
        }
        if accent.is_none() || scheme.is_none() {
            probe_xdg_portal(&mut accent, &mut scheme);
        }
        SystemTheme::from_parts(
            accent.unwrap_or((81, 162, 218)),
            scheme.unwrap_or(ThemeColorScheme::Dark),
            font.unwrap_or(GlyphFontStyle::ArcadeBlock),
        )
    }
    // Without it there is no desktop to probe. A renderer built this way must
    // not inherit the host's theme: the same job has to produce the same
    // pixels on every machine, or an export is not reproducible.
    #[cfg(not(feature = "product"))]
    {
        SystemTheme::from_parts(
            (81, 162, 218),
            ThemeColorScheme::Dark,
            GlyphFontStyle::ArcadeBlock,
        )
    }
}

#[cfg(feature = "product")]
fn probe_xdg_portal(accent: &mut Option<(u8, u8, u8)>, scheme: &mut Option<ThemeColorScheme>) {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        return;
    }
    let Ok(conn) = zbus::blocking::Connection::session() else {
        return;
    };
    let read_portal = |key: &str| {
        conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.Settings"),
            "ReadOne",
            &("org.freedesktop.appearance", key),
        )
        .ok()
        .and_then(|r| r.body().deserialize::<zbus::zvariant::OwnedValue>().ok())
    };
    if scheme.is_none()
        && let Some(val) = read_portal("color-scheme")
        && let Ok(code) = u32::try_from(val)
    {
        *scheme = Some(if code == 2 {
            ThemeColorScheme::Light
        } else {
            ThemeColorScheme::Dark
        });
    }
    if accent.is_none()
        && let Some(val) = read_portal("accent-color")
        && let Ok((r, g, b)) = <(f64, f64, f64)>::try_from(val)
        && [r, g, b].iter().all(|c| (0.0..=1.0).contains(c))
    {
        let u = |f: f64| (f * 255.0).round() as u8;
        *accent = Some((u(r), u(g), u(b)));
    }
}

#[cfg(any(feature = "product", test))]
pub(crate) fn probe_config_files(
    cfg: &Path,
    accent: &mut Option<(u8, u8, u8)>,
    scheme: &mut Option<ThemeColorScheme>,
    font: &mut Option<GlyphFontStyle>,
) {
    if let Ok(s) = fs::read_to_string(cfg.join("cosmic/com.system76.CosmicTheme.Mode/v1/is_dark")) {
        *scheme = Some(if s.trim() == "false" {
            ThemeColorScheme::Light
        } else {
            ThemeColorScheme::Dark
        });
    }
    let cosmic_acc = if *scheme == Some(ThemeColorScheme::Light) {
        "cosmic/com.system76.CosmicTheme.Light/v1/accent"
    } else {
        "cosmic/com.system76.CosmicTheme.Dark/v1/accent"
    };
    if let Ok(s) = fs::read_to_string(cfg.join(cosmic_acc)) {
        *accent = parse_cosmic_ron_rgb(&s).or(*accent);
    }
    if let Ok(s) = fs::read_to_string(cfg.join("cosmic/com.system76.CosmicTk/v1/monospace_font")) {
        *font = Some(GlyphFontStyle::parse(&s));
    }
    let confs = [
        "omarchy/current/theme/hyprland.conf",
        "superfedora/theme.conf",
        "hypr/hyprland.conf",
        "kdeglobals",
    ];
    for rel in confs {
        if let Ok(s) = fs::read_to_string(cfg.join(rel)) {
            *accent = accent.or_else(|| extract_conf_color(&s));
            if scheme.is_none() && s.to_ascii_lowercase().contains("light") {
                *scheme = Some(ThemeColorScheme::Light);
            }
            if font.is_none() && s.to_ascii_lowercase().contains("font") {
                *font = Some(GlyphFontStyle::parse(&s));
            }
        }
    }
    if let Ok(s) = fs::read_to_string(cfg.join("gtk-4.0/settings.ini"))
        .or_else(|_| fs::read_to_string(cfg.join("gtk-3.0/settings.ini")))
    {
        if scheme.is_none() {
            if s.contains("prefer-dark-theme=0") || s.contains("prefer-dark-theme=false") {
                *scheme = Some(ThemeColorScheme::Light);
            } else if s.contains("prefer-dark-theme=1") || s.contains("prefer-dark-theme=true") {
                *scheme = Some(ThemeColorScheme::Dark);
            }
        }
        if font.is_none()
            && let Some(line) = s.lines().find(|l| l.contains("gtk-font-name"))
        {
            *font = Some(GlyphFontStyle::parse(line));
        }
    }
}

pub fn parse_color_str(raw: &str) -> Option<(u8, u8, u8)> {
    let s = raw.trim().trim_start_matches('#').trim_start_matches("0x");
    if s.len() >= 6 {
        let hex = |a, b| u8::from_str_radix(&s[a..b], 16);
        if let (Ok(r), Ok(g), Ok(b)) = (hex(0, 2), hex(2, 4), hex(4, 6)) {
            return Some((r, g, b));
        }
    }
    let mut it = raw
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|p| p.parse::<u8>().ok());
    Some((it.next()?, it.next()?, it.next()?))
}

#[rustfmt::skip]
#[cfg(any(feature = "product", test))]
pub(crate) fn parse_cosmic_ron_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let mut it = raw.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').filter_map(|p| p.parse::<f32>().ok())
        .filter(|&f| (0.0..=255.0).contains(&f)).map(|f| (if f <= 1.0 { f * 255.0 } else { f }).round() as u8);
    Some((it.next()?, it.next()?, it.next()?))
}

#[cfg(any(feature = "product", test))]
pub(crate) fn extract_conf_color(raw: &str) -> Option<(u8, u8, u8)> {
    let keys = ["active_border", "accent", "AccentColor"];
    for line in raw.lines().map(str::trim).filter(|l| !l.starts_with('#')) {
        if let Some((lhs, rhs)) = line.split_once('=')
            && keys.iter().any(|k| lhs.trim().ends_with(k))
        {
            let inner = rhs.find("rgb(").and_then(|i| rhs[i + 4..].split_once(')'));
            if let Some(c) = parse_color_str(inner.map_or(rhs, |(hex, _)| hex)) {
                return Some(c);
            }
        }
    }
    None
}
