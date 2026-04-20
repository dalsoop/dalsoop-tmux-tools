mod apps;
mod cache;
mod cli;
mod command_runner;
mod config_io;
mod form;
mod list_view;
mod proxmox;
mod pve_handler;
mod render;
mod seed;
mod settings;
mod ssh;
mod tabs;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEventKind, MouseButton},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
};
use std::io;
use tmux_windowbar::config::template::Config;

use config_io::{load_config, save_and_apply};
use form::Form;
use list_view::ListView;
use proxmox::{ProxmoxServer, Container};
use settings::SettingItem;
use tabs::Tab;

// ─── palette ─────────────────────────────────────────────────────────────────
pub(crate) const BG:     Color = Color::Rgb(40,  44,  52);
pub(crate) const FG:     Color = Color::Rgb(171, 178, 191);
pub(crate) const GREEN:  Color = Color::Rgb(152, 195, 121);
pub(crate) const BLUE:   Color = Color::Rgb(97,  175, 239);
pub(crate) const RED:    Color = Color::Rgb(224, 108, 117);
pub(crate) const SUBTLE: Color = Color::Rgb(92,  99,  112);
pub(crate) const BORDER: Color = Color::Rgb(62,  68,  82);

// ─── mode ────────────────────────────────────────────────────────────────────
pub(crate) enum Mode {
    Normal,
    Editing,
    Confirming { label: String, idx: usize },
    SeedBrowse { list: ListView },
}

// ─── app ─────────────────────────────────────────────────────────────────────
pub(crate) struct App {
    pub(crate) tab:            Tab,
    pub(crate) ssh:            ListView,
    pub(crate) apps:           ListView,
    pub(crate) settings:       ListView,
    pub(crate) mode:           Mode,
    pub(crate) form:           Option<Form>,
    pub(crate) config:         Config,
    pub(crate) sb_config:      Option<tmux_sessionbar::config::template::Config>,
    pub(crate) setting_items:  Vec<SettingItem>,
    pub(crate) status_msg:     Option<String>,
    // Proxmox hierarchical state
    pub(crate) pve_list:       ListView,
    pub(crate) pve_depth:      usize, // 0=servers, 1=containers, 2=docker+ports
    pub(crate) pve_servers:    Vec<ProxmoxServer>,
    pub(crate) pve_sel_server: Option<usize>,
    pub(crate) pve_containers: Vec<Container>,
    pub(crate) pve_sel_ct:     Option<usize>,
    pub(crate) pve_detail:     Option<proxmox::DetailInfo>,
    pub(crate) pve_host_info:  Option<proxmox::HostInfo>,
}

impl App {
    fn new(config: Config) -> Self {
        let sb_config = config_io::load_sb_config().ok();
        let setting_items = settings::build_items(&config, sb_config.as_ref());
        let pve_servers = proxmox::get_servers(&config);
        let mut app = Self {
            tab: Tab::Ssh,
            ssh: ListView::new(),
            apps: ListView::new(),
            settings: ListView::new(),
            mode: Mode::Normal,
            form: None,
            config,
            sb_config,
            setting_items,
            status_msg: None,
            pve_list: ListView::new(),
            pve_depth: 0,
            pve_servers,
            pve_sel_server: None,
            pve_containers: Vec::new(),
            pve_sel_ct: None,
            pve_detail: None,
            pve_host_info: None,
        };
        app.sync_lengths();
        app
    }

    fn sync_lengths(&mut self) {
        self.ssh.set_len(self.config.ssh.len());
        self.apps.set_len(self.config.apps.len());
        self.settings.set_len(self.setting_items.len());
        self.sync_pve_list();
    }

    pub(crate) fn sync_pve_list(&mut self) {
        let len = match self.pve_depth {
            0 => self.pve_servers.len(),
            1 => self.pve_containers.len(),
            _ => self.pve_detail.as_ref().map_or(0, |d|
                d.docker.len() + d.ports.len() + d.snapshots.len() + 10 // header lines
            ),
        };
        self.pve_list.set_len(len);
    }

    fn reload_settings(&mut self) {
        self.setting_items = settings::build_items(&self.config, self.sb_config.as_ref());
        self.settings.set_len(self.setting_items.len());
    }

    pub(crate) fn current_list_mut(&mut self) -> &mut ListView {
        match self.tab {
            Tab::Ssh      => &mut self.ssh,
            Tab::Apps     => &mut self.apps,
            Tab::Proxmox  => &mut self.pve_list,
            Tab::Settings => &mut self.settings,
        }
    }

    fn move_down(&mut self) { self.current_list_mut().move_down(); }
    fn move_up(&mut self)   { self.current_list_mut().move_up(); }

    fn start_add(&mut self) {
        self.form = Some(match self.tab {
            Tab::Ssh      => ssh::add_form(),
            Tab::Apps     => apps::add_form(&self.config),
            Tab::Settings | Tab::Proxmox => return,
        });
        self.mode = Mode::Editing;
        self.status_msg = None;
    }

    fn start_edit(&mut self) {
        let form = match self.tab {
            Tab::Ssh => {
                let idx = match self.ssh.selected() { Some(i) => i, None => return };
                ssh::edit_form(&self.config, idx)
            }
            Tab::Apps => {
                let idx = match self.apps.selected() { Some(i) => i, None => return };
                apps::edit_form(&self.config, idx)
            }
            Tab::Settings => {
                let idx = match self.settings.selected() { Some(i) => i, None => return };
                let f = settings::edit_form(&self.setting_items, idx);
                if f.fields.is_empty() { return; } // Header, not editable
                f
            }
            Tab::Proxmox => return,
        };
        self.form = Some(form);
        self.mode = Mode::Editing;
        self.status_msg = None;
    }

    fn start_delete(&mut self) {
        match self.tab {
            Tab::Ssh => {
                let idx = match self.ssh.selected() { Some(i) => i, None => return };
                let label = self.config.ssh[idx].name.clone();
                self.mode = Mode::Confirming { label, idx };
            }
            Tab::Apps => {
                let idx = match self.apps.selected() { Some(i) => i, None => return };
                let label = self.config.apps[idx].command.clone();
                self.mode = Mode::Confirming { label, idx };
            }
            Tab::Settings | Tab::Proxmox => {}
        }
        self.status_msg = None;
    }

    fn confirm_delete(&mut self) {
        let (idx,) = match &self.mode {
            Mode::Confirming { idx, .. } => (*idx,),
            _ => return,
        };
        match self.tab {
            Tab::Ssh  => ssh::delete(&mut self.config, idx),
            Tab::Apps => apps::delete(&mut self.config, idx),
            Tab::Settings | Tab::Proxmox => {}
        }
        let _ = save_and_apply(&self.config);
        self.sync_lengths();
        self.mode = Mode::Normal;
        self.status_msg = Some("Deleted.".into());
    }

    /// Connect to SSH host: exit TUI, create/switch tmux session.
    /// Returns true to signal app exit.
    fn connect_ssh(&mut self) -> bool {
        let idx = match self.ssh.selected() {
            Some(i) => i,
            None => return false,
        };
        let entry = &self.config.ssh[idx];
        let session_name = format!("ssh-{}", entry.name);
        let ssh_target = if let Some(ref user) = entry.user {
            format!("{user}@{}", entry.host)
        } else {
            entry.host.clone()
        };

        // Check if session exists
        let has = std::process::Command::new("tmux")
            .args(["has-session", "-t", &format!("={session_name}")])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !has {
            // Create auto-reconnecting SSH session
            let ssh_cmd = format!(
                "while true; do ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=3 {ssh_target}; RC=$?; if [ $RC -eq 0 ]; then break; fi; echo '[연결 끊김 - 5초 후 재접속]'; sleep 5; done"
            );
            let _ = std::process::Command::new("tmux")
                .args(["new-session", "-d", "-s", &session_name, &ssh_cmd])
                .status();
        }

        // Switch to it
        let _ = std::process::Command::new("tmux")
            .args(["switch-client", "-t", &format!("={session_name}")])
            .status();

        // Re-render status bar
        let _ = std::process::Command::new("tmux-sessionbar")
            .args(["render-status", "left"])
            .status();

        true // exit TUI
    }

    // Old container methods removed — now use pve_* methods above

    fn open_seed_browser(&mut self) {
        let mut lv = ListView::new();
        lv.set_len(seed::SEEDS.len());
        self.mode = Mode::SeedBrowse { list: lv };
        self.status_msg = None;
    }

    fn seed_install(&mut self) {
        let idx = match &self.mode {
            Mode::SeedBrowse { list } => match list.selected() { Some(i) => i, None => return },
            _ => return,
        };
        if idx >= seed::SEEDS.len() { return; }
        let s = &seed::SEEDS[idx];

        match seed::check_install(s) {
            seed::InstallStep::AlreadyInstalled => {
                if !self.config.apps.iter().any(|a| a.command == s.command) {
                    self.add_seed_to_config(s);
                    self.status_msg = Some(format!("{} added to apps", s.command));
                } else {
                    self.status_msg = Some(format!("{} already in apps", s.command));
                }
            }
            seed::InstallStep::Ready(cmd) => {
                self.status_msg = Some(format!("Installing {} ({})...", s.command, cmd));
                if seed::install(s) {
                    self.add_seed_to_config(s);
                    self.status_msg = Some(format!("{} installed & added", s.command));
                } else {
                    self.status_msg = Some(format!("Failed to install {}", s.command));
                }
            }
            seed::InstallStep::NeedManager(mgr) => {
                // Ask to install the manager first via confirm
                let label = format!("install {mgr} first, then {}", s.command);
                self.mode = Mode::Confirming { label, idx };
            }
            seed::InstallStep::Unavailable => {
                self.status_msg = Some(format!("No package manager available for {}", s.command));
            }
        }
    }

    fn seed_install_with_manager(&mut self, idx: usize) {
        if idx >= seed::SEEDS.len() { return; }
        let s = &seed::SEEDS[idx];

        // Step 1: install package manager
        let mgr = match seed::check_install(s) {
            seed::InstallStep::NeedManager(m) => m,
            _ => { self.mode = Mode::Normal; return; }
        };
        self.status_msg = Some(format!("Installing {mgr}..."));
        if !seed::install_manager(mgr) {
            self.status_msg = Some(format!("Failed to install {mgr}"));
            self.mode = Mode::Normal;
            return;
        }

        // Step 2: install the app
        self.status_msg = Some(format!("Installing {}...", s.command));
        if seed::install(s) {
            self.add_seed_to_config(s);
            self.status_msg = Some(format!("{mgr} + {} installed & added", s.command));
        } else {
            self.status_msg = Some(format!("{mgr} installed, but {} failed", s.command));
        }
        self.mode = Mode::Normal;
    }

    fn add_seed_to_config(&mut self, s: &seed::SeedApp) {
        if !self.config.apps.iter().any(|a| a.command == s.command) {
            self.config.apps.push(tmux_windowbar::config::template::AppEntry {
                emoji: s.emoji.into(),
                command: s.command.into(),
                fg: s.fg.into(),
                bg: s.bg.into(),
                // mode = None — 글로벌 default_app_mode 따름. 특정 mode 강제 시에만 명시.
                mode: None,
            });
            let _ = save_and_apply(&self.config);
            self.sync_lengths();
        }
    }

    fn cancel(&mut self) {
        self.form = None;
        self.mode = Mode::Normal;
    }

    fn submit_form(&mut self) {
        let form = match self.form.take() {
            Some(f) => f,
            None => return,
        };
        match self.tab {
            Tab::Ssh  => ssh::apply_form(&mut self.config, &form),
            Tab::Apps => apps::apply_form(&mut self.config, &form),
            Tab::Settings => {
                settings::apply_form_wb(&mut self.config, &self.setting_items, &form);
                if let Some(ref mut sb) = self.sb_config {
                    settings::apply_form_sb(sb, &self.setting_items, &form);
                    let _ = config_io::save_and_apply_sb(sb);
                }
                self.reload_settings();
            }
            Tab::Proxmox => {}
        }
        let _ = save_and_apply(&self.config);
        self.sync_lengths();
        self.mode = Mode::Normal;
        self.status_msg = Some("Saved.".into());
    }

    fn handle_form_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let form = match &mut self.form {
            Some(f) => f,
            None => return,
        };
        match code {
            KeyCode::Char(c) => form.handle_char(c),
            KeyCode::Backspace => form.handle_backspace(),
            KeyCode::Tab => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    form.prev_field();
                } else {
                    form.next_field();
                }
            }
            KeyCode::BackTab => form.prev_field(),
            KeyCode::Enter => {
                // Drop the borrow so we can call submit_form
                let _ = form;
                self.submit_form();
            }
            KeyCode::Esc => self.cancel(),
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Esc if self.tab == Tab::Proxmox && self.pve_depth > 0 => self.pve_back(),
            KeyCode::Esc => return true,
            KeyCode::Tab | KeyCode::Char('\t') => {
                let next = (self.tab.index() + 1) % 5;
                self.tab = Tab::from_index(next);
                self.status_msg = None;
            }
            KeyCode::Char(n @ '1'..='9') => {
                let idx = (n as usize) - ('1' as usize);
                if idx < Tab::titles().len() {
                    self.tab = Tab::from_index(idx);
                    self.status_msg = None;
                }
            }
            // '4' is already covered by '1'..='9' above
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up => self.move_up(),
            KeyCode::Char('k') if !(self.tab == Tab::Proxmox && self.pve_depth == 0) => self.move_up(),
            KeyCode::Char('a') if self.tab != Tab::Proxmox => self.start_add(),
            KeyCode::Char('e') if self.tab != Tab::Proxmox => self.start_edit(),
            KeyCode::Enter if self.tab != Tab::Proxmox => self.start_edit(),
            KeyCode::Char('d') if self.tab != Tab::Proxmox => self.start_delete(),
            KeyCode::Char('i') if self.tab == Tab::Apps => self.open_seed_browser(),
            KeyCode::Char('c') if self.tab == Tab::Ssh => return self.connect_ssh(),
            // Proxmox: hierarchical navigation
            KeyCode::Enter | KeyCode::Right if self.tab == Tab::Proxmox => self.pve_enter(),
            KeyCode::Backspace | KeyCode::Left if self.tab == Tab::Proxmox => self.pve_back(),
            KeyCode::Char('c') if self.tab == Tab::Proxmox => return self.pve_connect(),
            KeyCode::Char('s') if self.tab == Tab::Proxmox => self.pve_start(),
            KeyCode::Char('x') if self.tab == Tab::Proxmox => self.pve_stop_confirm(),
            KeyCode::Char('r') if self.tab == Tab::Proxmox => self.pve_refresh(),
            KeyCode::Char('k') if self.tab == Tab::Proxmox && self.pve_depth == 0 => self.pve_install_key(),
            KeyCode::Char('D') if self.tab == Tab::Proxmox && self.pve_depth == 1 => self.pve_delete_confirm(),
            // Depth 2: docker/log actions
            KeyCode::Char('l') if self.tab == Tab::Proxmox && self.pve_depth == 1 => return self.pve_container_logs(),
            KeyCode::Char('l') if self.tab == Tab::Proxmox && self.pve_depth == 2 => return self.pve_docker_logs(),
            KeyCode::Char('e') if self.tab == Tab::Proxmox && self.pve_depth == 2 => return self.pve_docker_exec(),
            KeyCode::Char('s') if self.tab == Tab::Proxmox && self.pve_depth == 2 => self.pve_docker_action("start"),
            KeyCode::Char('x') if self.tab == Tab::Proxmox && self.pve_depth == 2 => self.pve_docker_action("stop"),
            _ => {}
        }
        false
    }

    fn handle_confirm_key(&mut self, code: KeyCode) -> bool {
        let idx = match &self.mode {
            Mode::Confirming { idx, .. } => *idx,
            _ => return false,
        };

        let label = match &self.mode {
            Mode::Confirming { label, .. } => label.clone(),
            _ => String::new(),
        };

        match code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if self.tab == Tab::Proxmox && label.starts_with("DELETE") {
                    self.pve_delete_execute();
                } else if self.tab == Tab::Proxmox {
                    self.pve_stop_execute();
                } else if self.tab == Tab::Apps {
                    // Could be seed manager install confirm
                    if idx < seed::SEEDS.len() && !seed::is_installed(seed::SEEDS[idx].command) {
                        self.seed_install_with_manager(idx);
                    } else {
                        self.confirm_delete();
                    }
                } else {
                    self.confirm_delete();
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Char('q') => return true,
            _ => {}
        }
        false
    }
}


// ─── entry point ─────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        return cli::cli_dispatch(&args[1..]);
    }

    // No args → TUI mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = load_config()?;
    let mut app = App::new(config);

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    // Track layout areas for mouse hit-testing
    let mut tab_area = Rect::default();
    let mut body_area = Rect::default();

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(size);
            tab_area = chunks[0];
            body_area = chunks[1];
            render::render(f, app);
        })?;

        if !event::poll(std::time::Duration::from_millis(200))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) => {
                let should_quit = if let Mode::SeedBrowse { ref mut list } = app.mode {
                    match key.code {
                        KeyCode::Down | KeyCode::Char('j') => { list.move_down(); false }
                        KeyCode::Up | KeyCode::Char('k') => { list.move_up(); false }
                        KeyCode::Enter => { app.seed_install(); false }
                        KeyCode::Esc | KeyCode::Char('q') => { app.mode = Mode::Normal; false }
                        _ => false,
                    }
                } else {
                    match app.mode {
                        Mode::Normal => app.handle_normal_key(key.code),
                        Mode::Editing => {
                            app.handle_form_key(key.code, key.modifiers);
                            false
                        }
                        Mode::Confirming { .. } => app.handle_confirm_key(key.code),
                        Mode::SeedBrowse { .. } => unreachable!(),
                    }
                };
                if should_quit {
                    break;
                }
            }
            Event::Mouse(mouse) => {
                if let Mode::Normal = app.mode {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let x = mouse.column;
                            let y = mouse.row;

                            // Tab bar click
                            if y >= tab_area.y && y < tab_area.y + tab_area.height {
                                let tab_titles = Tab::titles();
                                let mut offset = tab_area.x + 1;
                                for (i, title) in tab_titles.iter().enumerate() {
                                    let w = title.len() as u16 + 3; // " | " separator
                                    if x >= offset && x < offset + w {
                                        app.tab = Tab::from_index(i);
                                        app.status_msg = None;
                                        break;
                                    }
                                    offset += w;
                                }
                            }
                            // List body click — select item
                            else if y > body_area.y && y < body_area.y + body_area.height.saturating_sub(1) {
                                let row = (y - body_area.y - 1) as usize; // -1 for border
                                let list = app.current_list_mut();
                                if row < list.len() {
                                    list.select(row);
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => app.move_down(),
                        MouseEventKind::ScrollUp => app.move_up(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
