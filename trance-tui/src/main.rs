// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use trance_dbus::{TranceClient, daemon_available};

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActivePane {
    Settings,
    Screensavers,
}

struct App {
    client: Option<TranceClient>,
    daemon_running: bool,
    idle_enabled: bool,
    idle_timeout_mins: u32,
    render_scale: f32,
    show_fps_overlay: bool,
    active_saver: String,
    on_battery: bool,
    screensavers: Vec<String>,
    selected_saver_idx: usize,
    active_pane: ActivePane,
    selected_setting_idx: usize,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            client: None,
            daemon_running: false,
            idle_enabled: true,
            idle_timeout_mins: 5,
            render_scale: 1.0,
            show_fps_overlay: false,
            active_saver: "Random".to_string(),
            on_battery: false,
            screensavers: Vec::new(),
            selected_saver_idx: 0,
            active_pane: ActivePane::Settings,
            selected_setting_idx: 0,
        };
        app.refresh_state();
        app
    }

    fn refresh_state(&mut self) {
        self.daemon_running = daemon_available();
        if self.daemon_running {
            if let Ok(client) = TranceClient::connect() {
                if let Ok(status) = client.get_status() {
                    self.idle_enabled = status.idle_enabled;
                    self.idle_timeout_mins = status.idle_timeout_mins;
                    self.active_saver = if status.active_saver.is_empty() {
                        "Random".to_string()
                    } else {
                        status.active_saver
                    };
                    self.show_fps_overlay = status.show_fps_overlay;
                    self.render_scale = status.render_scale.parse::<f32>().unwrap_or(1.0);
                    self.on_battery = status.inhibited; // Simple fallback or query battery status
                }
                if let Ok(savers) = client.list_savers() {
                    self.screensavers = savers;
                }
                self.client = Some(client);
            }
        } else {
            self.client = None;
            self.screensavers = trance_runner::discovery::detect_screensavers();
        }

        // Check live battery status directly
        let sys = trance_runner::toolkit::sys_info::get_system_info();
        self.on_battery = sys.power_status.contains("Battery");
    }

    fn toggle_daemon(&mut self) {
        if self.daemon_running {
            // Stop only — keep the unit enabled for next login.
            let _ = Command::new("systemctl")
                .args(["--user", "stop", "trance-daemon.service"])
                .status();
        } else {
            // enable --now so upgrades/logins keep the daemon alive.
            let sys_status = Command::new("systemctl")
                .args(["--user", "enable", "--now", "trance-daemon.service"])
                .status();
            let success = sys_status.map(|s| s.success()).unwrap_or(false);
            if !success {
                let _ = Command::new("trance-daemon").arg("daemon").spawn();
            }
        }
        std::thread::sleep(Duration::from_millis(350));
        self.refresh_state();
    }

    fn toggle_idle(&mut self) {
        if let Some(ref client) = self.client {
            if self.idle_enabled {
                let _ = client.disable();
            } else {
                let _ = client.enable();
            }
        }
        self.refresh_state();
    }

    fn adjust_timeout(&mut self, delta: i32) {
        let mut val = self.idle_timeout_mins as i32 + delta;
        val = val.clamp(1, 240);
        self.idle_timeout_mins = val as u32;
        if let Some(ref client) = self.client {
            let _ = client.set_timeout(self.idle_timeout_mins);
        }
    }

    fn adjust_scale(&mut self, delta: f32) {
        let mut val = self.render_scale + delta;
        val = val.clamp(0.25, 1.0);
        self.render_scale = val;
        if let Some(ref client) = self.client {
            let _ = client.set_render_scale(self.render_scale);
        }
    }

    fn toggle_fps(&mut self) {
        if let Some(ref client) = self.client {
            let _ = client.set_show_fps_overlay(!self.show_fps_overlay);
        }
        self.refresh_state();
    }

    fn select_saver(&mut self) {
        if let Some(ref client) = self.client {
            let name = if self.selected_saver_idx == 0 {
                ""
            } else {
                &self.screensavers[self.selected_saver_idx - 1]
            };
            let _ = client.set_saver(name);
        }
        self.refresh_state();
    }

    fn preview_saver(&mut self) {
        let saver = if self.selected_saver_idx == 0 {
            if self.screensavers.is_empty() {
                "beams".to_string()
            } else {
                self.screensavers[0].clone()
            }
        } else {
            self.screensavers[self.selected_saver_idx - 1].clone()
        };

        // Prefer live daemon overlay; start service if needed.
        if !self.daemon_running {
            self.toggle_daemon();
        }

        let mut started_via_dbus = false;
        if self.daemon_running {
            // Refresh client after possible start.
            if self.client.is_none() {
                self.refresh_state();
            }
            if let Some(ref client) = self.client
                && client.preview(&saver).is_ok()
            {
                started_via_dbus = true;
            }
        }
        if !started_via_dbus {
            // Packaged fallback (not the unshipped trance-runner binary).
            let _ = Command::new("trance-daemon")
                .args(["run-plugin", &saver])
                .status();
        }
    }
}

fn display_saver_name(raw: &str) -> String {
    if raw.eq_ignore_ascii_case("random") || raw == "Random selection" {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut cap = true;
    for ch in raw.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
            cap = true;
            continue;
        }
        if cap {
            out.extend(ch.to_uppercase());
            cap = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error running TUI: {err}");
    }
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('d'))
                {
                    return Ok(());
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Tab => {
                        app.active_pane = match app.active_pane {
                            ActivePane::Settings => ActivePane::Screensavers,
                            ActivePane::Screensavers => ActivePane::Settings,
                        };
                    }
                    KeyCode::Up => {
                        match app.active_pane {
                            ActivePane::Settings => {
                                if app.selected_setting_idx > 0 {
                                    app.selected_setting_idx -= 1;
                                }
                            }
                            ActivePane::Screensavers => {
                                if app.selected_saver_idx > 0 {
                                    app.selected_saver_idx -= 1;
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        match app.active_pane {
                            ActivePane::Settings => {
                                if app.selected_setting_idx < 4 {
                                    app.selected_setting_idx += 1;
                                }
                            }
                            ActivePane::Screensavers => {
                                if app.selected_saver_idx <= app.screensavers.len() {
                                    app.selected_saver_idx += 1;
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        if app.active_pane == ActivePane::Settings {
                            match app.selected_setting_idx {
                                2 => app.adjust_timeout(-1),
                                3 => app.adjust_scale(-0.05),
                                _ => {}
                            }
                        }
                    }
                    KeyCode::Right => {
                        if app.active_pane == ActivePane::Settings {
                            match app.selected_setting_idx {
                                2 => app.adjust_timeout(1),
                                3 => app.adjust_scale(0.05),
                                _ => {}
                            }
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        match app.active_pane {
                            ActivePane::Settings => match app.selected_setting_idx {
                                0 => app.toggle_daemon(),
                                1 => app.toggle_idle(),
                                4 => app.toggle_fps(),
                                _ => {}
                            },
                            ActivePane::Screensavers => {
                                if key.code == KeyCode::Enter {
                                    app.select_saver();
                                }
                            }
                        }
                    }
                    KeyCode::Char('p') => {
                        app.preview_saver();
                        // Re-enable raw mode in case preview terminal mode disrupted it
                        let _ = enable_raw_mode();
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.refresh_state();
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title Bar
    let battery_status = if app.on_battery {
        " [Battery · 30 FPS]".yellow().bold()
    } else {
        "".into()
    };
    let daemon_hint = if app.daemon_running {
        " · daemon live".green()
    } else {
        " · daemon stopped (Space on Daemon starts + enables)".dark_gray()
    };
    let title = Line::from(vec![
        " Trance Screensaver ".cyan().bold(),
        battery_status,
        daemon_hint,
    ]);
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let title_paragraph = Paragraph::new(title).block(title_block);
    f.render_widget(title_paragraph, chunks[0]);

    // Columns Layout
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    // Left Pane: Settings
    let mut settings_list = vec![
        format!(
            "Daemon Service:       {}",
            if app.daemon_running {
                "RUNNING (auto-start on)"
            } else {
                "STOPPED"
            }
        ),
        format!(
            "Idle Activation:      {}",
            if app.idle_enabled { "ON" } else { "OFF" }
        ),
        format!("Idle Timeout:         {} min", app.idle_timeout_mins),
        format!("Render Scale:         {:.0}%", app.render_scale * 100.0),
        format!(
            "FPS Overlay:          {}",
            if app.show_fps_overlay { "ON" } else { "OFF" }
        ),
    ];

    // Highlight active element in settings
    let mut settings_items = Vec::new();
    for (idx, text) in settings_list.iter_mut().enumerate() {
        let mut style = Style::default();
        if app.active_pane == ActivePane::Settings && app.selected_setting_idx == idx {
            style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            *text = format!("> {text}");
        } else {
            *text = format!("  {text}");
        }
        settings_items.push(ListItem::new(text.clone()).style(style));
    }

    let settings_block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.active_pane == ActivePane::Settings {
            Color::Yellow
        } else {
            Color::DarkGray
        }));
    let settings_widget = List::new(settings_items).block(settings_block);
    f.render_widget(settings_widget, columns[0]);

    // Right Pane: Screensavers list
    let mut saver_items = vec![ListItem::new(format!(
        "{} Random",
        if app.active_saver == "Random" { "*" } else { " " }
    ))];

    for s in &app.screensavers {
        let prefix = if app.active_saver == *s { "*" } else { " " };
        saver_items.push(ListItem::new(format!(
            "{prefix} {}",
            display_saver_name(s)
        )));
    }

    // Apply Highlight
    let mut state = ListState::default();
    state.select(Some(app.selected_saver_idx));

    let savers_block = Block::default()
        .title(" Screensavers ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.active_pane == ActivePane::Screensavers {
            Color::Yellow
        } else {
            Color::DarkGray
        }));
    let savers_widget = List::new(saver_items)
        .block(savers_block)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(savers_widget, columns[1], &mut state);

    // Help Bar
    let help_text = match app.active_pane {
        ActivePane::Settings => {
            " [Tab] Pane | [Space/Enter] Toggle | [←/→] Timeout/Scale | [q] Quit  ·  Daemon on = enable --now"
        }
        ActivePane::Screensavers => {
            " [Tab] Pane | [↑/↓] Navigate | [Enter] Set Active | [p] Preview | [q] Quit"
        }
    };
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let help_paragraph = Paragraph::new(help_text).block(help_block);
    f.render_widget(help_paragraph, chunks[2]);
}
