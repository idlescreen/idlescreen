use super::*;

#[test]
fn sanitize_accepts_clean_names() {
    assert_eq!(
        sanitize_saver_name("security"),
        Some("security".to_string())
    );
    assert_eq!(
        sanitize_saver_name("/path/to/beams.scr"),
        Some("beams".to_string())
    );
    assert_eq!(
        sanitize_saver_name("screensaver-storm"),
        Some("storm".to_string())
    );
}

#[test]
fn sanitize_rejects_bad_names() {
    assert!(sanitize_saver_name("evil;rm -rf /").is_none());
    assert_eq!(
        sanitize_saver_name("../../etc/passwd"),
        Some("passwd".to_string())
    );
    assert_eq!(
        sanitize_saver_name("not-a-real-saver"),
        Some("not-a-real-saver".to_string())
    );
    assert!(sanitize_saver_name("").is_none());
}

#[test]
fn allowlist_blocks_unknown_savers() {
    assert!(!is_allowed_saver("not-a-real-saver"));
    assert!(!is_allowed_saver("passwd"));
    assert!(is_allowed_saver("cosmos"));
    assert!(resolve_saver_binary("evil-plugin", &LaunchMode::Daemon).is_err());
}

#[test]
fn allowlist_is_complete() {
    assert_eq!(ALLOWED_SAVERS.len(), 11);
    assert!(ALLOWED_SAVERS.contains(&"beams"));
    assert!(ALLOWED_SAVERS.contains(&"waves"));
    assert!(ALLOWED_SAVERS.contains(&"loom"));
    assert!(ALLOWED_SAVERS.contains(&"relay"));
}

#[test]
fn is_trusted_plugin_path_rejects_nonexistent() {
    let p = std::path::Path::new("/nonexistent/foo.so");
    assert!(!is_trusted_plugin_path(p, &[]));
}

#[test]
fn is_trusted_plugin_path_rejects_when_not_in_trusted_dirs() {
    let p = std::path::Path::new("/nonexistent/foo.so");
    let trusted = vec![std::path::PathBuf::from("/also/nonexistent")];
    assert!(!is_trusted_plugin_path(p, &trusted));
}

#[test]
fn sanitize_rejects_shell_metacharacters() {
    assert!(sanitize_saver_name("name|pipe").is_none());
    assert!(sanitize_saver_name("name&bg").is_none());
    assert!(sanitize_saver_name("name$dollar").is_none());
    assert!(sanitize_saver_name("name space").is_none());
}

#[test]
fn sanitize_accepts_alphanumeric_and_dash() {
    assert_eq!(sanitize_saver_name("abc-123"), Some("abc-123".to_string()));
    assert_eq!(sanitize_saver_name("ABC"), Some("ABC".to_string()));
}

#[test]
fn sanitize_strips_screensaver_prefix_only() {
    assert_eq!(
        sanitize_saver_name("screensaver-bursts"),
        Some("bursts".to_string())
    );
    assert_eq!(
        sanitize_saver_name("screensaver-screensaver-x"),
        Some("screensaver-x".to_string())
    );
}

#[test]
fn is_allowed_saver_rejects_path_separators() {
    assert!(!is_allowed_saver("../beams"));
    assert!(!is_allowed_saver("dir\\beams"));
    assert!(!is_allowed_saver("/beams"));
}

#[test]
fn resolve_saver_binary_rejects_path_separators() {
    assert!(resolve_saver_binary("../beams", &LaunchMode::Daemon).is_err());
    assert!(resolve_saver_binary("dir\\beams", &LaunchMode::Daemon).is_err());
    assert!(resolve_saver_binary("/abs/beams", &LaunchMode::Daemon).is_err());
}

#[test]
fn resolve_saver_binary_rejects_unknown_saver() {
    assert!(resolve_saver_binary("not-in-allowlist", &LaunchMode::Daemon).is_err());
    assert!(resolve_saver_binary("", &LaunchMode::Daemon).is_err());
}

#[test]
fn launch_mode_clone_eq() {
    let a = LaunchMode::Daemon;
    let b = a.clone();
    assert_eq!(a, b);
    let c = LaunchMode::Preview;
    assert_ne!(a, c);
}

#[test]
fn plugin_error_display_includes_context() {
    let err = PluginError::NotAllowed("evil".to_string());
    let s = err.to_string();
    assert!(s.contains("evil"));
    let err = PluginError::PathTraversal;
    assert!(err.to_string().contains(".."));
}
