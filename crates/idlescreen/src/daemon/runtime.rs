// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Runtime environment detection: bare host, generic container, Podman, or COSMIC cage.

use std::path::Path;

/// The kind of runtime environment the daemon is executing in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEnvironment {
    Host,
    GenericContainer,
    PodmanContainer,
    CosmicCage,
}

impl RuntimeEnvironment {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Host => "Host",
            Self::GenericContainer => "GenericContainer",
            Self::PodmanContainer => "PodmanContainer",
            Self::CosmicCage => "CosmicCage",
        }
    }
}

impl std::fmt::Display for RuntimeEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Detects the current runtime environment from the live system.
pub fn detect_environment() -> RuntimeEnvironment {
    detect_with(
        |key| std::env::var(key).ok(),
        |path| Path::new(path).exists(),
        |path| std::fs::read_to_string(path).ok(),
    )
}

/// Probes the environment through injectable closures so unit tests can
/// simulate any host/container/cage combination without touching the real fs.
fn detect_with(
    get_env: impl Fn(&str) -> Option<String>,
    path_exists: impl Fn(&str) -> bool,
    read_file: impl Fn(&str) -> Option<String>,
) -> RuntimeEnvironment {
    let truthy = |value: Option<String>| {
        value.is_some_and(|v| {
            let t = v.trim();
            !t.is_empty() && t != "0" && !t.eq_ignore_ascii_case("false")
        })
    };
    let eq_ignore = |v: &Option<String>, needle: &str| {
        v.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(needle))
    };
    let has_cage = |v: &Option<String>| {
        v.as_deref()
            .is_some_and(|s| s.to_ascii_lowercase().contains("cosmic-cage"))
    };
    let container = get_env("container");
    let systemd_container = read_file("/run/systemd/container").map(|c| c.trim().to_string());
    let mut cgroups = String::new();
    for path in ["/proc/1/cgroup", "/proc/self/cgroup"] {
        if let Some(content) = read_file(path) {
            cgroups.push_str(&content);
            cgroups.push('\n');
        }
    }

    // a) COSMIC cage wins over every container marker.
    let cosmic_desktop =
        has_cage(&get_env("XDG_SESSION_DESKTOP")) || has_cage(&get_env("XDG_CURRENT_DESKTOP"));
    let cosmic_cage = truthy(get_env("COSMIC_CAGE"))
        || truthy(get_env("COSMIC_SANDBOX"))
        || eq_ignore(&container, "cosmic-cage")
        || cosmic_desktop
        || path_exists("/.cosmic-cage")
        || path_exists("/run/.cosmic-cage");
    if cosmic_cage {
        return RuntimeEnvironment::CosmicCage;
    }

    // b) Podman markers.
    let podman = path_exists("/run/.containerenv")
        || eq_ignore(&container, "podman")
        || systemd_container.as_deref() == Some("podman")
        || cgroups.contains("libpod");
    if podman {
        return RuntimeEnvironment::PodmanContainer;
    }

    // c) Generic container markers.
    let generic = path_exists("/.dockerenv")
        || path_exists("/.flatpak-info")
        || container.as_deref().is_some_and(|c| !c.trim().is_empty())
        || systemd_container.as_deref().is_some_and(|c| !c.is_empty())
        || cgroups.contains("docker")
        || cgroups.contains("kubepods")
        || cgroups.contains("containerd")
        || cgroups.contains("/lxc/");
    if generic {
        return RuntimeEnvironment::GenericContainer;
    }

    // d) Bare metal or VM host.
    RuntimeEnvironment::Host
}

#[cfg(test)]
mod tests {
    use super::RuntimeEnvironment::*;
    use super::*;

    #[derive(Default)]
    struct FakeSystem {
        env: Vec<(String, String)>,
        paths: Vec<String>,
        files: Vec<(String, String)>,
    }

    fn to_map(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn sys(env: &[(&str, &str)], paths: &[&str], files: &[(&str, &str)]) -> FakeSystem {
        FakeSystem {
            env: to_map(env),
            paths: paths.iter().map(ToString::to_string).collect(),
            files: to_map(files),
        }
    }

    impl FakeSystem {
        fn detect(&self) -> RuntimeEnvironment {
            let env = &self.env;
            let paths = &self.paths;
            let files = &self.files;
            detect_with(
                |key| env.iter().find(|p| p.0 == key).map(|p| p.1.clone()),
                |path| paths.iter().any(|p| p == path),
                |path| files.iter().find(|p| p.0 == path).map(|p| p.1.clone()),
            )
        }
    }

    fn env_is(env: &[(&str, &str)], want: RuntimeEnvironment) {
        assert_eq!(sys(env, &[], &[]).detect(), want);
    }

    fn path_is(paths: &[&str], want: RuntimeEnvironment) {
        assert_eq!(sys(&[], paths, &[]).detect(), want);
    }

    fn file_is(files: &[(&str, &str)], want: RuntimeEnvironment) {
        assert_eq!(sys(&[], &[], files).detect(), want);
    }

    #[test]
    fn cosmic_cage_takes_priority_over_container_markers() {
        let paths = &["/.dockerenv", "/run/.containerenv"];
        let env = &[("COSMIC_CAGE", "1"), ("container", "docker")];
        assert_eq!(sys(env, paths, &[]).detect(), CosmicCage);
    }

    #[test]
    fn cosmic_cage_markers() {
        env_is(&[("COSMIC_SANDBOX", "true")], CosmicCage);
        env_is(&[("container", "cosmic-cage")], CosmicCage);
        let cage = "cosmic-cage";
        env_is(&[("XDG_SESSION_DESKTOP", cage)], CosmicCage);
        env_is(&[("XDG_CURRENT_DESKTOP", cage)], CosmicCage);
        path_is(&["/.cosmic-cage"], CosmicCage);
        path_is(&["/run/.cosmic-cage"], CosmicCage);
        // Disabled markers must not trigger.
        env_is(&[("COSMIC_CAGE", "0")], Host);
        env_is(&[("COSMIC_SANDBOX", "false")], Host);
        env_is(&[("COSMIC_CAGE", "")], Host);
    }

    #[test]
    fn podman_container_detection() {
        path_is(&["/run/.containerenv"], PodmanContainer);
        env_is(&[("container", "PODMAN")], PodmanContainer);
        let sysd = "/run/systemd/container";
        file_is(&[(sysd, "podman\n")], PodmanContainer);
        let cg1 = "/proc/1/cgroup";
        file_is(&[(cg1, "0::/libpod")], PodmanContainer);
        let cg = "/proc/self/cgroup";
        file_is(&[(cg, "0::/libpod-x")], PodmanContainer);
        // Podman beats generic container markers.
        let both = &["/run/.containerenv", "/.dockerenv"];
        path_is(both, PodmanContainer);
    }

    #[test]
    fn generic_container_detection() {
        path_is(&["/.dockerenv"], GenericContainer);
        path_is(&["/.flatpak-info"], GenericContainer);
        env_is(&[("container", "docker")], GenericContainer);
        let sysd = "/run/systemd/container";
        file_is(&[(sysd, "docker")], GenericContainer);
        let cg1 = "/proc/1/cgroup";
        file_is(&[(cg1, "0::/kubepods/pod")], GenericContainer);
        let cg = "/proc/self/cgroup";
        file_is(&[(cg, "0::/docker/abc")], GenericContainer);
        file_is(&[(cg1, "0::/containerd/x")], GenericContainer);
        file_is(&[(cg, "0::/lxc/ct")], GenericContainer);
    }

    #[test]
    fn display_formatting() {
        assert_eq!(Host.to_string(), "Host");
        assert_eq!(GenericContainer.as_str(), "GenericContainer");
        assert_eq!(PodmanContainer.to_string(), "PodmanContainer");
        assert_eq!(CosmicCage.to_string(), "CosmicCage");
        assert_eq!(Host.as_str(), "Host");
        assert_eq!(format!("{CosmicCage}"), "CosmicCage");
    }

    #[test]
    fn host_is_default() {
        assert_eq!(FakeSystem::default().detect(), Host);
        env_is(&[("container", "   ")], Host);
        let sysd = "/run/systemd/container";
        assert_eq!(sys(&[], &[], &[(sysd, "  \n")]).detect(), Host);
    }

    #[test]
    fn live_detection_returns_a_variant() {
        assert!(matches!(
            detect_environment(),
            Host | GenericContainer | PodmanContainer | CosmicCage
        ));
    }
}
