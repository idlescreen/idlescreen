// SPDX-License-Identifier: Apache-2.0
//! Common test harness probes and mocks for end-to-end testing.

pub mod env_mock;
pub mod notify_probe;
pub mod stat_probe;
pub mod syslog_probe;

pub use env_mock::{
    InitEnvironment, InitSystem, detect_mock_init, repo_root, safe_exec_supervisor,
    service_template_path, supervisor_binary,
};
pub use notify_probe::{NotifySocketProbe, PipeReadinessProbe, emit_notify_socket};
pub use stat_probe::{
    ClockSample, compute_cpu_seconds, detect_suspend_divergence, parse_cgroup_v2_path,
    parse_proc_stat_cpu_ticks, read_real_clocks,
};
pub use syslog_probe::{
    ParsedSyslog, RollingFileProbe, SyslogProbe, format_rfc3164, parse_rfc3164_datagram,
};
