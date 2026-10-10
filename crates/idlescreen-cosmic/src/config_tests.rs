// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::ThemeConfig;

#[test]
fn defaults_are_sane() {
    let d = ThemeConfig::defaults();
    assert_eq!(d.idle_timeout_mins, 2);
    assert_eq!(d.active_saver.as_deref(), Some("ascii"));
    assert!(d.idle_enabled);
    assert!(d.render_scale.is_none());
}

#[test]
fn parses_known_keys() {
    let yaml = r##"
# comment
accent_color: "#FF0000"
idle_timeout_mins: 15
theme_idx: 2
active_saver: "cosmos"
idle_enabled: false
show_fps_overlay: true
render_scale: 0.5
unknown_key: ignored
"##;
    let c = ThemeConfig::from_yaml_content(yaml);
    assert_eq!(c.accent_color, "#FF0000");
    assert_eq!(c.idle_timeout_mins, 15);
    assert_eq!(c.theme_idx, 2);
    assert_eq!(c.active_saver.as_deref(), Some("cosmos"));
    assert!(!c.idle_enabled);
    assert!(c.show_fps_overlay);
    assert_eq!(c.render_scale, Some(0.5));
}

#[test]
fn render_scale_null_maps_to_none() {
    let c = ThemeConfig::from_yaml_content("render_scale: null\n");
    assert!(c.render_scale.is_none());
    let c = ThemeConfig::from_yaml_content("render_scale: \"null\"\n");
    assert!(c.render_scale.is_none());
}

#[test]
fn save_merges_preserving_foreign_keys() {
    let existing = "# user note\ntheme: \"synthwave\"\nstrict_control: true\nidle_timeout_mins: 5\n[saver]\nbeams.speed: 3\n";
    let mut c = ThemeConfig::defaults();
    c.idle_timeout_mins = 15;
    let out = super::merge_preserving(existing, &mut c.rendered_fields());
    assert!(out.contains("# user note"));
    assert!(out.contains("theme: \"synthwave\""));
    assert!(out.contains("strict_control: true"));
    assert!(out.contains("[saver]\nbeams.speed: 3"));
    assert!(out.contains("idle_timeout_mins: 15"));
    assert!(out.contains("render_scale: null"));
}

#[test]
fn active_saver_none_clears() {
    let c = ThemeConfig::from_yaml_content("active_saver: none\n");
    assert_eq!(c.active_saver, None);
    let c2 = ThemeConfig::from_yaml_content("active_saver: \"\"\n");
    assert_eq!(c2.active_saver, None);
}

#[test]
fn bad_values_keep_defaults() {
    let c = ThemeConfig::from_yaml_content(
        "idle_timeout_mins: not-a-number\nrender_scale: xyz\nidle_enabled: maybe\n",
    );
    let d = ThemeConfig::defaults();
    assert_eq!(c.idle_timeout_mins, d.idle_timeout_mins);
    assert_eq!(c.render_scale, d.render_scale);
    assert_eq!(c.idle_enabled, d.idle_enabled);
}

#[test]
fn quoted_and_unquoted_values() {
    let c = ThemeConfig::from_yaml_content("active_saver: 'storm'\naccent_color: #00FF00\n");
    assert_eq!(c.active_saver.as_deref(), Some("storm"));
    assert_eq!(c.accent_color, "#00FF00");
}
