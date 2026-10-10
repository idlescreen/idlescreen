// SPDX-License-Identifier: Apache-2.0
//! Tier 4 Scenario 6: Installer multi-init detection and graceful deployment.

use crate::harness::{
    InitEnvironment, InitSystem, detect_mock_init, safe_exec_supervisor,
};
use std::fs;

#[test]
fn test_scenario_install_on_runit_host() {
    // 1. Host has runit installed (/etc/runit/runsvdir)
    let env = InitEnvironment::new(InitSystem::Runit);
    let init = detect_mock_init(env.root());
    assert_eq!(init, InitSystem::Runit);

    // 2. Package installer copies runit service template
    let service_dir = env.root().join("etc/sv/idle-daemon");
    fs::create_dir_all(&service_dir).unwrap();
    let run_script = service_dir.join("run");
    fs::write(&run_script, "#!/bin/sh\nexec idle-daemon daemon\n").unwrap();

    assert!(run_script.exists());

    // 3. Supervisor starts service via sv without systemctl
    let bin_dir = env.bin_dir();
    fs::write(bin_dir.join("sv"), "#!/bin/sh\nexit 0\n").unwrap();

    let res = safe_exec_supervisor(InitSystem::Runit, "start", "idle-daemon", Some(&bin_dir));
    assert_eq!(res, Ok("success:sv:start:idle-daemon".to_string()));
}

#[test]
fn test_scenario_install_on_openrc_host() {
    // 1. Host has OpenRC installed (/run/openrc)
    let env = InitEnvironment::new(InitSystem::OpenRc);
    let init = detect_mock_init(env.root());
    assert_eq!(init, InitSystem::OpenRc);

    // 2. Package installer copies openrc runscript
    let service_dir = env.root().join("etc/init.d");
    fs::create_dir_all(&service_dir).unwrap();
    let init_script = service_dir.join("idle-daemon");
    fs::write(&init_script, "#!/sbin/openrc-run\ncommand=/usr/bin/idle-daemon\n").unwrap();

    assert!(init_script.exists());

    // 3. Supervisor starts service via rc-service
    let bin_dir = env.bin_dir();
    fs::write(bin_dir.join("rc-service"), "#!/bin/sh\nexit 0\n").unwrap();

    let res = safe_exec_supervisor(InitSystem::OpenRc, "start", "idle-daemon", Some(&bin_dir));
    assert_eq!(res, Ok("success:rc-service:start:idle-daemon".to_string()));
}
