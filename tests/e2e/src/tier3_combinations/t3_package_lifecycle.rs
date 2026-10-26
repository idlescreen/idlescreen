// SPDX-License-Identifier: Apache-2.0
//! Tier 3: Pairwise combination — Package Detection + Service Template Deployment.

use crate::harness::{
    InitEnvironment, InitSystem, detect_mock_init, repo_root, safe_exec_supervisor,
    service_template_path, supervisor_binary,
};
use std::fs;

const ALL_INITS: [InitSystem; 6] = [
    InitSystem::Systemd,
    InitSystem::OpenRc,
    InitSystem::Runit,
    InitSystem::Dinit,
    InitSystem::S6,
    InitSystem::Standalone,
];

#[test]
fn detection_matches_every_mocked_init_system() {
    for init in ALL_INITS {
        let env = InitEnvironment::new(init);
        assert_eq!(
            detect_mock_init(env.root()),
            init,
            "the init probe must recognise a {init:?} host"
        );
    }
}

#[test]
fn detection_prefers_systemd_when_markers_overlap() {
    // A host can carry leftovers from another init system. The probe order
    // is part of the contract: systemd wins, because it is the only one
    // whose marker is a real runtime directory.
    let env = InitEnvironment::new(InitSystem::OpenRc);
    fs::create_dir_all(env.root().join("run/systemd/system")).ok();
    assert_eq!(detect_mock_init(env.root()), InitSystem::Systemd);
}

#[test]
fn every_supported_init_system_ships_a_service_template() {
    let root = repo_root();
    for init in ALL_INITS {
        let Some(rel) = service_template_path(init) else {
            assert_eq!(
                init,
                InitSystem::Standalone,
                "only standalone mode runs without a service template"
            );
            continue;
        };
        let path = root.join(rel);
        assert!(
            path.exists(),
            "{init:?} is advertised but {rel} is not in the repository"
        );
        assert!(
            fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false),
            "{rel} is empty — a template that ships nothing starts nothing"
        );
    }
}

#[test]
fn supervisor_binary_is_distinct_per_init_system() {
    let mut seen = std::collections::HashSet::new();
    for init in ALL_INITS {
        let binary = supervisor_binary(init);
        assert!(!binary.is_empty(), "{init:?} has no supervisor binary");
        assert!(
            seen.insert(binary),
            "{init:?} reuses the supervisor binary {binary}"
        );
    }
}

#[test]
fn supervisor_call_falls_back_when_the_binary_is_absent() {
    for init in ALL_INITS {
        let out = safe_exec_supervisor(init, "start", "idlescreen", None)
            .expect("a missing supervisor must fall back, not fail");
        assert_eq!(out, "fallback_direct_spawn:start:idlescreen");
    }
}

#[test]
fn supervisor_call_uses_the_supervisor_when_it_is_present() {
    for init in ALL_INITS {
        let env = InitEnvironment::new(init);
        let bin_dir = env.bin_dir();
        let binary = supervisor_binary(init);
        fs::write(bin_dir.join(binary), b"#!/bin/sh\nexit 0\n").ok();
        let out = safe_exec_supervisor(init, "start", "idlescreen", Some(&bin_dir))
            .expect("the supervisor call must succeed");
        assert_eq!(out, format!("success:{binary}:start:idlescreen"));
    }
}
