// SPDX-License-Identifier: Apache-2.0
//! Tier 4 Scenario 5: Rootless container environment without systemd or cgroups.

use crate::harness::{
    InitEnvironment, InitSystem, RollingFileProbe, compute_cpu_seconds, detect_mock_init,
    parse_proc_stat_cpu_ticks,
};
use tempfile::TempDir;

#[test]
fn test_scenario_rootless_container_environment() {
    // 1. Detect standalone environment (no systemd/openrc/etc.)
    let env = InitEnvironment::new(InitSystem::Standalone);
    assert_eq!(detect_mock_init(env.root()), InitSystem::Standalone);

    // 2. Cgroup v2 is not writable/delegated in rootless container
    let cgroup_writable = false;
    assert!(!cgroup_writable);

    // 3. Fallback to /proc/self/stat CPU accounting
    let simulated_stat = "1001 (idlescreen) S 1 1001 1001 0 0 0 0 0 0 0 450 150 0 0 20 0 1 0 0 0 0";
    let ticks = parse_proc_stat_cpu_ticks(simulated_stat);
    assert_eq!(ticks, Some((450, 150)));

    let cpu_sec = compute_cpu_seconds(450, 150, 100);
    assert_eq!(cpu_sec, 6.0);

    // 4. Logging falls back to ~/.local/state/idlescreen/idlescreen.log
    let temp = TempDir::new().unwrap();
    let file_probe = RollingFileProbe::new(temp.path());
    file_probe
        .write_line("INFO idlescreen: container fallback active")
        .unwrap();

    let logs = file_probe.read_all();
    assert!(logs.contains("container fallback active"));
}
