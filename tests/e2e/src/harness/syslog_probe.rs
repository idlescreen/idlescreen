// SPDX-License-Identifier: Apache-2.0
//! Syslog RFC 3164 datagram and rolling log file probe.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedSyslog {
    pub priority: u8,
    pub facility: u8,
    pub severity: u8,
    pub content: String,
}

pub struct SyslogProbe {
    _temp_dir: TempDir,
    socket_path: PathBuf,
    listener: UnixDatagram,
}

impl Default for SyslogProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl SyslogProbe {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("create temp dir for syslog socket");
        let socket_path = temp_dir.path().join("dev_log.sock");
        let listener = UnixDatagram::bind(&socket_path).expect("bind dev_log socket");
        listener
            .set_read_timeout(Some(Duration::from_millis(500)))
            .ok();

        Self {
            _temp_dir: temp_dir,
            socket_path,
            listener,
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn recv_syslog(&self) -> Option<ParsedSyslog> {
        let mut buf = [0u8; 1024];
        match self.listener.recv(&mut buf) {
            Ok(n) if n > 0 => {
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                parse_rfc3164_datagram(&text)
            }
            _ => None,
        }
    }
}

pub fn format_rfc3164(facility: u8, severity: u8, tag: &str, pid: u32, msg: &str) -> String {
    let pri = facility * 8 + severity;
    let ts = "Oct 09 12:00:00";
    let host = "localhost";
    format!("<{pri}>{ts} {host} {tag}[{pid}]: {msg}")
}

pub fn parse_rfc3164_datagram(raw: &str) -> Option<ParsedSyslog> {
    if !raw.starts_with('<') {
        return None;
    }
    let end_pri = raw.find('>')?;
    let pri_str = &raw[1..end_pri];
    let priority = pri_str.parse::<u8>().ok()?;
    let facility = priority / 8;
    let severity = priority % 8;
    let content = raw[end_pri + 1..].to_string();

    Some(ParsedSyslog {
        priority,
        facility,
        severity,
        content,
    })
}

pub struct RollingFileProbe {
    log_path: PathBuf,
}

impl RollingFileProbe {
    pub fn new(base: &Path) -> Self {
        let state_dir = base.join(".local/state/idlescreen");
        fs::create_dir_all(&state_dir).ok();
        let log_path = state_dir.join("idle-daemon.log");
        Self { log_path }
    }

    pub fn path(&self) -> &Path {
        &self.log_path
    }

    pub fn write_line(&self, line: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        writeln!(file, "{line}")?;
        file.flush()
    }

    pub fn read_all(&self) -> String {
        fs::read_to_string(&self.log_path).unwrap_or_default()
    }

    pub fn rotate(&self, max_size_bytes: u64) -> bool {
        let meta = match fs::metadata(&self.log_path) {
            Ok(m) => m,
            Err(_) => return false,
        };
        if meta.len() >= max_size_bytes {
            let backup = self.log_path.with_extension("log.1");
            fs::rename(&self.log_path, backup).is_ok()
        } else {
            false
        }
    }
}
