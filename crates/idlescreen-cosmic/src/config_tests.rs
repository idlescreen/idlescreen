// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

use super::ThemeConfig;

#[test]
fn defaults_are_sane() {
    let d = ThemeConfig::defaults();
    assert_eq!(d.idle_timeout_mins, 5);
    assert_eq!(d.active_saver.as_deref(), Some("ascii"));
    assert!(d.idle_enabled);
    assert!(d.render_scale.is_none());
}

#[test]
fn parses_known_toml_and_yaml_keys() {
    let toml = r##"
# comment
accent_color = "#FF0000"
idle_timeout_secs = 900
theme_idx = 2
active_saver = "cosmos"
idle_enabled = false
show_fps_overlay = true
render_scale = 0.5
unknown_key = "ignored"
[saver.ascii]
fps = 60
"##;
    let c = ThemeConfig::from_yaml_content(toml);
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
    let c = ThemeConfig::from_yaml_content("render_scale = \"null\"\n");
    assert!(c.render_scale.is_none());
    let c = ThemeConfig::from_yaml_content("render_scale: null\n");
    assert!(c.render_scale.is_none());
}

#[test]
fn save_merges_preserving_foreign_keys_and_sections() {
    let existing = "# user note\ntheme = \"synthwave\"\nstrict_control = true\nidle_timeout_secs = 300\n[saver.ascii]\nfps = 60\n";
    let mut c = ThemeConfig::defaults();
    c.idle_timeout_mins = 15;
    let out = super::merge_preserving(existing, &mut c.rendered_fields());
    assert!(out.contains("# user note"));
    assert!(out.contains("theme = \"synthwave\""));
    assert!(out.contains("strict_control = true"));
    assert!(out.contains("[saver.ascii]\nfps = 60"));
    assert!(out.contains("idle_timeout_secs = 900"));
    assert!(out.contains("render_scale = \"null\""));
}

#[test]
fn active_saver_none_clears() {
    let c = ThemeConfig::from_yaml_content("active_saver = \"none\"\n");
    assert_eq!(c.active_saver, None);
    let c2 = ThemeConfig::from_yaml_content("active_saver = \"\"\n");
    assert_eq!(c2.active_saver, None);
}

#[test]
fn bad_values_keep_defaults() {
    let c = ThemeConfig::from_yaml_content(
        "idle_timeout_secs = not-a-number\nrender_scale = xyz\nidle_enabled = maybe\n",
    );
    let d = ThemeConfig::defaults();
    assert_eq!(c.idle_timeout_mins, d.idle_timeout_mins);
    assert_eq!(c.render_scale, d.render_scale);
    assert_eq!(c.idle_enabled, d.idle_enabled);
}

#[test]
fn quoted_and_unquoted_values() {
    let c = ThemeConfig::from_yaml_content("active_saver = 'aurora'\naccent_color: #00FF00\n");
    assert_eq!(c.active_saver.as_deref(), Some("aurora"));
    assert_eq!(c.accent_color, "#00FF00");
}

#[test]
fn parses_and_merges_daemon_config_toml_sections() {
    let daemon_toml = "[general]\nidle_enabled = false\nidle_timeout_secs = 600\n\n[saver]\nactive = \"plasma\"\n\n[saver.ascii]\nfps = 60\n";
    let mut c = ThemeConfig::from_yaml_content(daemon_toml);
    assert!(!c.idle_enabled);
    assert_eq!(c.idle_timeout_mins, 10);
    assert_eq!(c.active_saver.as_deref(), Some("plasma"));

    c.idle_timeout_mins = 12;
    c.active_saver = Some("tunnel".to_string());
    let merged = super::merge_preserving(daemon_toml, &mut c.rendered_fields());
    assert!(merged.contains("[general]\nidle_enabled = false\nidle_timeout_secs = 720"));
    assert!(merged.contains("[saver]\nactive = \"tunnel\""));
    assert!(merged.contains("[saver.ascii]\nfps = 60"));
}
