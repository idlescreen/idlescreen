use std::fs;
use std::path::PathBuf;

use std::time::Instant;

use super::particles::Particle;

#[derive(Debug, Clone)]
pub struct AppState {
    pub savers: Vec<String>,
    pub selected_idx: usize,
    pub idle_timeout_mins: u32,
    pub accent_color: (u8, u8, u8),
    pub dark_mode: bool,
    pub should_quit: bool,
    pub status_message: String,
    pub status_ttl_sec: u32,
    pub daemon_running: bool,
    pub theme_idx: usize,
    pub quote_idx: usize,
    pub particles: Vec<Particle>,
    pub selection_start: Option<(u16, u16)>,
    pub selection_end: Option<(u16, u16)>,
    pub selection_pending_copy: bool,
    pub copied_toast: Option<String>,
    pub copied_toast_until: Option<Instant>,
    pub active_saver: Option<String>,
    pub idle_enabled: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let savers = trance_runner::discovery::detect_screensavers();
        let is_dark = trance_runner::toolkit::sys_info::query_dark_mode();
        let default_accent = Self::get_accent_by_index(0, is_dark);
        let q_idx = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize;

        let mut state = Self {
            savers,
            selected_idx: 0,
            idle_timeout_mins: 5,
            accent_color: default_accent,
            dark_mode: is_dark,
            should_quit: false,
            status_message: "".to_string(),
            status_ttl_sec: 0,
            daemon_running: false,
            theme_idx: 0,
            quote_idx: q_idx,
            particles: Vec::new(),
            selection_start: None,
            selection_end: None,
            selection_pending_copy: false,
            copied_toast: None,
            copied_toast_until: None,
            active_saver: Some("beams".to_string()),
            idle_enabled: true,
        };

        state.load_config();

        // Always query current OS theme value and use the associated accent
        let is_dark = trance_runner::toolkit::sys_info::query_dark_mode();
        state.dark_mode = is_dark;
        state.accent_color = Self::get_accent_by_index(state.theme_idx, is_dark);

        state.check_daemon_running();
        state
    }

    pub fn get_accent_by_index(idx: usize, is_dark: bool) -> (u8, u8, u8) {
        let i = idx % 5;
        if is_dark {
            match i {
                0 => (0, 191, 255),  // navy -> electric blue
                1 => (186, 85, 211), // violet -> neon purple/magenta
                2 => (0, 255, 200),  // teal -> neon mint/teal
                3 => (255, 0, 127),  // fuchsia -> hot pink
                _ => (255, 110, 0),  // rust -> neon orange
            }
        } else {
            match i {
                0 => (46, 80, 144),  // navy
                1 => (107, 63, 160), // violet
                2 => (0, 128, 128),  // teal
                3 => (192, 64, 128), // fuchsia
                _ => (210, 105, 30), // rust
            }
        }
    }

    fn get_config_path() -> Option<PathBuf> {
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg_config.is_empty() {
                return Some(PathBuf::from(xdg_config).join("local76").join("theme.yaml"));
            }
        }
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("local76")
                .join("theme.yaml"),
        )
    }

    pub fn load_config(&mut self) {
        if let Some(path) = Self::get_config_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                let mut loaded_theme_idx = None;
                let mut loaded_accent = None;
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some(idx) = line.find(':') {
                        let key = line[..idx].trim();
                        let val = line[idx + 1..].trim().trim_matches('"').trim_matches('\'');
                        match key {
                            "accent_color" => {
                                if val.starts_with('#') && val.len() == 7 {
                                    if let (Ok(r), Ok(g), Ok(b)) = (
                                        u8::from_str_radix(&val[1..3], 16),
                                        u8::from_str_radix(&val[3..5], 16),
                                        u8::from_str_radix(&val[5..7], 16),
                                    ) {
                                        loaded_accent = Some((r, g, b));
                                        self.accent_color = (r, g, b);
                                    }
                                }
                            }
                            "dark_mode" | "is_dark_mode" => {
                                if let Ok(b) = val.parse::<bool>() {
                                    self.dark_mode = b;
                                }
                            }
                            "idle_timeout_mins" => {
                                if let Ok(n) = val.parse::<u32>() {
                                    self.idle_timeout_mins = n;
                                }
                            }
                            "theme_idx" => {
                                if let Ok(idx) = val.parse::<usize>() {
                                    loaded_theme_idx = Some(idx);
                                }
                            }
                            "active_saver" => {
                                if !val.is_empty() && val != "none" {
                                    self.active_saver = Some(val.to_string());
                                }
                            }
                            "idle_enabled" => {
                                if let Ok(b) = val.parse::<bool>() {
                                    self.idle_enabled = b;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Align theme_idx if it was loaded directly, or derive it from accent_color
                if let Some(idx) = loaded_theme_idx {
                    self.theme_idx = idx;
                } else if let Some(acc) = loaded_accent {
                    let mut found = None;
                    for i in 0..5 {
                        if Self::get_accent_by_index(i, self.dark_mode) == acc
                            || Self::get_accent_by_index(i, !self.dark_mode) == acc
                        {
                            found = Some(i);
                            break;
                        }
                    }
                    if let Some(idx) = found {
                        self.theme_idx = idx;
                    }
                }
            }
        }
    }

    pub fn save_config(&mut self) -> std::io::Result<()> {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let hex_color = format!(
                "#{:02X}{:02X}{:02X}",
                self.accent_color.0, self.accent_color.1, self.accent_color.2
            );
            let active_str = self.active_saver.as_deref().unwrap_or("none");
            let content = format!(
                "# local76 themes and settings\n\
                 accent_color: \"{}\"\n\
                 # dark_mode is auto-detected from system\n\
                 idle_timeout_mins: {}\n\
                 theme_idx: {}\n\
                 active_saver: \"{}\"\n\
                 idle_enabled: {}\n",
                hex_color, self.idle_timeout_mins, self.theme_idx, active_str, self.idle_enabled
            );
            fs::write(&path, content)?;
        } else {
            self.status_message = "failed to determine config directory".to_string();
            self.status_ttl_sec = 5;
        }
        Ok(())
    }

    pub fn check_daemon_running(&mut self) {
        // Under Linux, we can check if the systemd user service is running or check pid file
        let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            std::path::PathBuf::from(runtime_dir).join("trance-daemon.pid")
        } else {
            std::env::temp_dir().join("trance-daemon.pid")
        };
        if pid_path.exists() {
            if let Ok(pid_str) = fs::read_to_string(&pid_path) {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    // Check if process exists by sending signal 0 (libc)
                    unsafe {
                        if libc::kill(pid, 0) == 0 {
                            self.daemon_running = true;
                            return;
                        }
                    }
                }
            }
            let _ = fs::remove_file(pid_path);
        }
        self.daemon_running = false;
    }
}
