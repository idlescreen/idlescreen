/// Cross-platform "where are we running" descriptor.
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub logo_text: String,
    pub kernel: String,
    pub hostname: String,
    pub cpu: String,
    pub uptime_secs: u64,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub mem_used_pct: f32,
    pub cpu_usage_pct: f32,
    pub power_status: String,
    pub disk_summary: String,
    pub gpus: String,
    pub monitors: String,
}

impl Default for SystemInfo {
    fn default() -> Self {
        if Self::export_mode_enabled() {
            return Self::export_fixture();
        }
        let os = std::env::var("TRANCE_OS_NAME").unwrap_or_else(|_| {
            let mut temp_os = "Linux".to_string();
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        let val = line.split('=').nth(1).unwrap_or("").trim_matches('"');
                        if !val.is_empty() {
                            temp_os = val.to_string();
                            break;
                        }
                    }
                }
            }
            temp_os
        });

        let logo_text = std::env::var("TRANCE_LOGO_TEXT").unwrap_or_else(|_| {
            let mut temp_logo = "Linux".to_string();
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        let val = line.split('=').nth(1).unwrap_or("").trim_matches('"');
                        if !val.is_empty() {
                            temp_logo = val.to_string();
                            break;
                        }
                    }
                }
            }
            temp_logo
        });

        let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());

        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            os,
            logo_text,
            kernel,
            hostname,
            cpu: "CPU".to_string(),
            uptime_secs: 0,
            mem_used_mb: 1,
            mem_total_mb: 2,
            mem_used_pct: 50.0,
            cpu_usage_pct: 0.0,
            power_status: "AC".to_string(),
            disk_summary: "disks".to_string(),
            gpus: "GPU".to_string(),
            monitors: "1 monitor".to_string(),
        }
    }
}

impl SystemInfo {
    /// Stable fixture for offline export (`TRANCE_EXPORT_MODE=1` / render).
    pub fn export_fixture() -> Self {
        Self {
            os: "IdleScreen Export".into(),
            logo_text: "IdleScreen".into(),
            kernel: "export".into(),
            hostname: "export-host".into(),
            cpu: "CPU".into(),
            uptime_secs: 0,
            mem_used_mb: 1024,
            mem_total_mb: 2048,
            mem_used_pct: 50.0,
            cpu_usage_pct: 0.0,
            power_status: "AC".into(),
            disk_summary: "disk".into(),
            gpus: "GPU".into(),
            monitors: "1 monitor".into(),
        }
    }

    /// True when host should prefer deterministic export fixtures.
    pub fn export_mode_enabled() -> bool {
        matches!(
            std::env::var("TRANCE_EXPORT_MODE").ok().as_deref(),
            Some("1") | Some("true") | Some("TRUE")
        ) || std::env::var_os("RENDER_SEED").is_some()
    }
}
