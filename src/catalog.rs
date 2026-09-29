// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! The IdleScreen component catalog — the single source of truth the
//! router dispatches on. Runnable components exec their binary;
//! managed components have no user-facing binary (they are installed,
//! versioned, and checked only).

use std::process::Command;

/// A product component in the org.
pub struct Component {
    /// Name users type after `idlescreen`.
    pub name: &'static str,
    /// System package that provides the component.
    pub package: &'static str,
    /// Binary the component execs, if runnable.
    pub binary: Option<&'static str>,
    /// One-line description for listings.
    pub desc: &'static str,
}

pub const COMPONENTS: &[Component] = &[
    Component {
        name: "cli",
        package: "idle-cli",
        binary: Some("idle-cli"),
        desc: "protocol commands — preview, stop, list, status, config",
    },
    Component {
        name: "tui",
        package: "idle-tui",
        binary: Some("idle-tui"),
        desc: "runtime configuration TUI",
    },
    Component {
        name: "studio",
        package: "idle-studio",
        binary: Some("idle-studio"),
        desc: "offline render queue TUI",
    },
    Component {
        name: "cosmic",
        package: "idle-cosmic",
        binary: Some("idlescreen-applet"),
        desc: "COSMIC panel applet",
    },
    Component {
        name: "savers",
        package: "idle-savers",
        binary: None,
        desc: "all official saver plugins (metapackage)",
    },
    Component {
        name: "runtime",
        package: "idle-daemon",
        binary: None,
        desc: "daemon + plugin runtime (the engine underneath)",
    },
];

/// Alternate names accepted for components.
const ALIASES: &[(&str, &str)] = &[
    ("daemon", "runtime"),
    ("idle", "runtime"),
    ("plugins", "savers"),
];

pub fn lookup(name: &str) -> Option<&'static Component> {
    let name = ALIASES
        .iter()
        .find(|(a, _)| *a == name)
        .map_or(name, |(_, t)| *t);
    COMPONENTS.iter().find(|c| c.name == name)
}

/// True if `bin` resolves on PATH.
pub fn binary_installed(bin: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
}

/// Query the installed version of a system package (rpm or dpkg).
pub fn package_version(pkg: &str) -> Option<String> {
    let out = Command::new("rpm")
        .args(["-q", "--qf", "%{VERSION}-%{RELEASE}", pkg])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if out.as_deref().is_some_and(|s| !s.is_empty()) {
        return out;
    }
    Command::new("dpkg-query")
        .args(["-W", "-f", "${Version}", pkg])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Whether a component is present on this machine (binary on PATH, or
/// package installed for managed components).
pub fn installed(c: &Component) -> bool {
    match c.binary {
        Some(bin) => binary_installed(bin),
        None => package_version(c.package).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_component_resolves() {
        for c in COMPONENTS {
            assert_eq!(lookup(c.name).map(|x| x.name), Some(c.name));
        }
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(lookup("daemon").map(|c| c.name), Some("runtime"));
        assert_eq!(lookup("idle").map(|c| c.name), Some("runtime"));
        assert_eq!(lookup("plugins").map(|c| c.name), Some("savers"));
    }

    #[test]
    fn unknown_is_none() {
        assert!(lookup("preview").is_none());
        assert!(lookup("bogus").is_none());
        assert!(lookup("").is_none());
    }

    #[test]
    fn cli_verbs_are_not_components() {
        // Anything the router claims natively shadows the same idle-cli
        // verb — the catalog must not collide with the protocol surface.
        for verb in ["status", "config", "list", "doctor", "update", "stop"] {
            assert!(lookup(verb).is_none(), "{verb} must route to idle-cli");
        }
    }

    /// The teardown script this package ships must remove the whole
    /// product stack — keep it in sync with the component catalog.
    /// (Contract ported from the retired packages/metapackages/idlescreen.)
    #[test]
    fn remove_product_stack_lists_match() {
        const STACK: &[&str] = &[
            "idle-cosmic",
            "idle-tui",
            "idle-cli",
            "idle-savers",
            "idle-saver-aurora",
            "idle-saver-beams",
            "idle-saver-bursts",
            "idle-saver-chaos",
            "idle-saver-cosmos",
            "idle-saver-glyphs",
            "idle-saver-gnats",
            "idle-saver-hearth",
            "idle-saver-radar",
            "idle-saver-ripple",
            "idle-saver-storm",
            "idle-daemon",
        ];
        let script = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/libexec/remove-product-stack.sh"
        ))
        .expect("read remove-product-stack.sh");
        for pkg in STACK {
            assert!(
                script.contains(pkg),
                "remove-product-stack.sh must list {pkg}"
            );
        }
        assert!(script.contains("idlescreen.repo"));
        assert!(script.contains("idlescreen.list"));
    }

    #[test]
    fn names_and_packages_are_distinct() {
        let mut pkgs: Vec<_> = COMPONENTS.iter().map(|c| c.package).collect();
        pkgs.sort_unstable();
        pkgs.dedup();
        assert_eq!(pkgs.len(), COMPONENTS.len());
    }
}
