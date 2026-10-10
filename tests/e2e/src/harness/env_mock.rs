// SPDX-License-Identifier: Apache-2.0
//! Mock environment fixtures for multi-init systems.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitSystem {
    Systemd,
    OpenRc,
    Runit,
    Dinit,
    S6,
    Standalone,
}

pub struct InitEnvironment {
    temp_dir: TempDir,
    init: InitSystem,
}

impl InitEnvironment {
    pub fn new(init: InitSystem) -> Self {
        let temp_dir = TempDir::new().expect("create temp dir for init mock");
        let root = temp_dir.path();

        match init {
            InitSystem::Systemd => {
                fs::create_dir_all(root.join("run/systemd/system")).ok();
            }
            InitSystem::OpenRc => {
                fs::create_dir_all(root.join("run/openrc")).ok();
            }
            InitSystem::Runit => {
                fs::create_dir_all(root.join("etc/runit/runsvdir")).ok();
            }
            InitSystem::Dinit => {
                fs::create_dir_all(root.join("run/dinit")).ok();
            }
            InitSystem::S6 => {
                fs::create_dir_all(root.join("run/service")).ok();
            }
            InitSystem::Standalone => {}
        }

        Self { temp_dir, init }
    }

    pub fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn target_init(&self) -> InitSystem {
        self.init
    }

    pub fn bin_dir(&self) -> PathBuf {
        let dir = self.root().join("bin");
        fs::create_dir_all(&dir).ok();
        dir
    }
}

pub fn detect_mock_init(root: &Path) -> InitSystem {
    if root.join("run/systemd/system").exists() {
        InitSystem::Systemd
    } else if root.join("run/openrc").exists() {
        InitSystem::OpenRc
    } else if root.join("etc/runit/runsvdir").exists() {
        InitSystem::Runit
    } else if root.join("run/dinit").exists() {
        InitSystem::Dinit
    } else if root.join("run/service").exists() {
        InitSystem::S6
    } else {
        InitSystem::Standalone
    }
}

pub fn safe_exec_supervisor(
    init: InitSystem,
    verb: &str,
    unit: &str,
    bin_path: Option<&Path>,
) -> Result<String, String> {
    let binary = match init {
        InitSystem::Systemd => "systemctl",
        InitSystem::OpenRc => "rc-service",
        InitSystem::Runit => "sv",
        InitSystem::Dinit => "dinitctl",
        InitSystem::S6 => "s6-rc",
        InitSystem::Standalone => "idle-daemon",
    };

    let bin_exists = if let Some(dir) = bin_path {
        dir.join(binary).exists()
    } else {
        false
    };

    if !bin_exists {
        // Safe execution: Do NOT early abort with NotFound io error.
        // Fall back gracefully to direct spawn indicator.
        return Ok(format!("fallback_direct_spawn:{verb}:{unit}"));
    }

    Ok(format!("success:{binary}:{verb}:{unit}"))
}
