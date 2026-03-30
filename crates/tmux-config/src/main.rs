mod apps;
mod config_io;
mod form;
mod list_view;
mod proxmox;
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
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use std::io;
use tmux_windowbar::config::template::Config;

use config_io::{load_config, save_and_apply};
use form::Form;
use list_view::ListView;
use proxmox::{ProxmoxServer, Container, DockerContainer};
use settings::SettingItem;
use tabs::Tab;

// ─── palette ─────────────────────────────────────────────────────────────────
const BG:     Color = Color::Rgb(40,  44,  52);
const FG:     Color = Color::Rgb(171, 178, 191);
const GREEN:  Color = Color::Rgb(152, 195, 121);
const BLUE:   Color = Color::Rgb(97,  175, 239);
const RED:    Color = Color::Rgb(224, 108, 117);
const SUBTLE: Color = Color::Rgb(92,  99,  112);
const BORDER: Color = Color::Rgb(62,  68,  82);

// ─── mode ────────────────────────────────────────────────────────────────────
enum Mode {
    Normal,
    Editing,
    Confirming { label: String, idx: usize },
}

// ─── app ─────────────────────────────────────────────────────────────────────
struct App {
    tab:            Tab,
    ssh:            ListView,
    apps:           ListView,
    settings:       ListView,
    mode:           Mode,
    form:           Option<Form>,
    config:         Config,
    setting_items:  Vec<SettingItem>,
    status_msg:     Option<String>,
    // Proxmox hierarchical state
    pve_list:       ListView,
    pve_depth:      usize, // 0=servers, 1=containers, 2=docker+ports
    pve_servers:    Vec<ProxmoxServer>,
    pve_sel_server: Option<usize>,
    pve_containers: Vec<Container>,
    pve_sel_ct:     Option<usize>,
    pve_docker:     Vec<DockerContainer>,
    pve_ports:      Vec<String>,
}

impl App {
    fn new(config: Config) -> Self {
        let setting_items = settings::build_items(&config);
        let pve_servers = proxmox::get_servers(&config);
        let mut app = Self {
            tab: Tab::Ssh,
            ssh: ListView::new(),
            apps: ListView::new(),
            settings: ListView::new(),
            mode: Mode::Normal,
            form: None,
            config,
            setting_items,
            status_msg: None,
            pve_list: ListView::new(),
            pve_depth: 0,
            pve_servers,
            pve_sel_server: None,
            pve_containers: Vec::new(),
            pve_sel_ct: None,
            pve_docker: Vec::new(),
            pve_ports: Vec::new(),
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

    fn sync_pve_list(&mut self) {
        let len = match self.pve_depth {
            0 => self.pve_servers.len(),
            1 => self.pve_containers.len(),
            _ => self.pve_docker.len() + self.pve_ports.len(),
        };
        self.pve_list.set_len(len);
    }

    fn reload_settings(&mut self) {
        self.setting_items = settings::build_items(&self.config);
        self.settings.set_len(self.setting_items.len());
    }

    fn current_list_mut(&mut self) -> &mut ListView {
        match self.tab {
            Tab::Ssh      => &mut self.ssh,
            Tab::Apps     => &mut self.apps,
            Tab::Proxmox  => &mut self.pve_list,
            Tab::Settings => &mut self.settings,
        }
    }

    // Proxmox drill-down
    fn pve_enter(&mut self) {
        match self.pve_depth {
            0 => {
                // Enter server → fetch containers
                let idx = match self.pve_list.selected() { Some(i) => i, None => return };
                if idx >= self.pve_servers.len() { return; }
                self.pve_sel_server = Some(idx);
                self.status_msg = Some("Loading containers...".into());
                self.pve_containers = proxmox::fetch_containers(&self.pve_servers[idx]);
                self.pve_depth = 1;
                self.sync_pve_list();
                if self.pve_containers.is_empty() {
                    self.status_msg = Some("No containers found or unreachable".into());
                } else {
                    self.status_msg = Some(format!("{} containers", self.pve_containers.len()));
                }
            }
            1 => {
                // Enter container → fetch docker + ports
                let idx = match self.pve_list.selected() { Some(i) => i, None => return };
                if idx >= self.pve_containers.len() { return; }
                let server_idx = self.pve_sel_server.unwrap_or(0);
                let server = &self.pve_servers[server_idx];
                let ct = &self.pve_containers[idx];
                if ct.kind != "lxc" {
                    self.status_msg = Some("Docker inspection only for LXC".into());
                    return;
                }
                self.pve_sel_ct = Some(idx);
                self.status_msg = Some("Loading docker & ports...".into());
                self.pve_docker = proxmox::fetch_docker(server, ct.vmid);
                self.pve_ports = proxmox::fetch_ports(server, ct.vmid);
                self.pve_depth = 2;
                self.sync_pve_list();
                let msg = format!("{} docker, {} ports", self.pve_docker.len(), self.pve_ports.len());
                self.status_msg = Some(msg);
            }
            _ => {}
        }
    }

    fn pve_back(&mut self) {
        match self.pve_depth {
            2 => {
                self.pve_depth = 1;
                self.pve_docker.clear();
                self.pve_ports.clear();
                self.sync_pve_list();
                self.status_msg = None;
            }
            1 => {
                self.pve_depth = 0;
                self.pve_containers.clear();
                self.pve_sel_server = None;
                self.sync_pve_list();
                self.status_msg = None;
            }
            _ => {}
        }
    }

    fn pve_refresh(&mut self) {
        self.pve_servers = proxmox::get_servers(&self.config);
        match self.pve_depth {
            0 => {
                self.sync_pve_list();
                self.status_msg = Some(format!("{} servers", self.pve_servers.len()));
            }
            1 => {
                let idx = self.pve_sel_server.unwrap_or(0);
                if idx < self.pve_servers.len() {
                    self.pve_containers = proxmox::fetch_containers(&self.pve_servers[idx]);
                    self.sync_pve_list();
                    self.status_msg = Some(format!("{} containers", self.pve_containers.len()));
                }
            }
            2 => {
                let si = self.pve_sel_server.unwrap_or(0);
                let ci = self.pve_sel_ct.unwrap_or(0);
                if si < self.pve_servers.len() && ci < self.pve_containers.len() {
                    let server = &self.pve_servers[si];
                    let vmid = self.pve_containers[ci].vmid;
                    self.pve_docker = proxmox::fetch_docker(server, vmid);
                    self.pve_ports = proxmox::fetch_ports(server, vmid);
                    self.sync_pve_list();
                }
            }
            _ => {}
        }
    }

    fn pve_console(&mut self) -> bool {
        if self.pve_depth != 1 { return false; }
        let idx = match self.pve_list.selected() { Some(i) => i, None => return false };
        if idx >= self.pve_containers.len() { return false; }
        let si = self.pve_sel_server.unwrap_or(0);
        if si >= self.pve_servers.len() { return false; }
        let server = &self.pve_servers[si];
        let ct = &self.pve_containers[idx];
        let session_name = format!("ct-{}", ct.vmid);
        let cmd = match proxmox::console_cmd(server, ct) {
            Some(c) => c,
            None => {
                self.status_msg = Some("Console not available for API-only servers".into());
                return false;
            }
        };

        let has = std::process::Command::new("tmux")
            .args(["has-session", "-t", &format!("={session_name}")])
            .status().map(|s| s.success()).unwrap_or(false);
        if !has {
            let _ = std::process::Command::new("tmux")
                .args(["new-session", "-d", "-s", &session_name, &cmd])
                .status();
        }
        let _ = std::process::Command::new("tmux")
            .args(["switch-client", "-t", &format!("={session_name}")])
            .status();
        true
    }

    fn pve_start(&mut self) {
        if self.pve_depth != 1 { return; }
        let idx = match self.pve_list.selected() { Some(i) => i, None => return };
        if idx >= self.pve_containers.len() { return; }
        let si = self.pve_sel_server.unwrap_or(0);
        let ct = self.pve_containers[idx].clone();
        if ct.status == "running" {
            self.status_msg = Some(format!("{} already running", ct.name));
            return;
        }
        self.status_msg = Some(format!("Starting {}...", ct.name));
        proxmox::start_container(&self.pve_servers[si], &ct);
        self.pve_refresh();
    }

    fn pve_stop_confirm(&mut self) {
        if self.pve_depth != 1 { return; }
        let idx = match self.pve_list.selected() { Some(i) => i, None => return };
        if idx >= self.pve_containers.len() { return; }
        let ct = &self.pve_containers[idx];
        if ct.status != "running" {
            self.status_msg = Some(format!("{} not running", ct.name));
            return;
        }
        let label = format!("stop {}", ct.name);
        self.mode = Mode::Confirming { label, idx };
    }

    fn pve_stop_execute(&mut self) {
        let idx = match &self.mode { Mode::Confirming { idx, .. } => *idx, _ => return };
        if idx >= self.pve_containers.len() { self.mode = Mode::Normal; return; }
        let si = self.pve_sel_server.unwrap_or(0);
        let ct = self.pve_containers[idx].clone();
        self.status_msg = Some(format!("Stopping {}...", ct.name));
        proxmox::stop_container(&self.pve_servers[si], &ct);
        self.mode = Mode::Normal;
        self.pve_refresh();
    }

    fn pve_breadcrumb(&self) -> String {
        match self.pve_depth {
            0 => "Proxmox Servers".into(),
            1 => {
                let si = self.pve_sel_server.unwrap_or(0);
                let name = self.pve_servers.get(si).map(|s| s.name.as_str()).unwrap_or("?");
                format!("{name} > Containers")
            }
            _ => {
                let si = self.pve_sel_server.unwrap_or(0);
                let ci = self.pve_sel_ct.unwrap_or(0);
                let sname = self.pve_servers.get(si).map(|s| s.name.as_str()).unwrap_or("?");
                let cname = self.pve_containers.get(ci).map(|c| c.name.as_str()).unwrap_or("?");
                format!("{sname} > {cname} > Docker/Ports")
            }
        }
    }

    fn move_down(&mut self) { self.current_list_mut().move_down(); }
    fn move_up(&mut self)   { self.current_list_mut().move_up(); }

    fn start_add(&mut self) {
        self.form = Some(match self.tab {
            Tab::Ssh      => ssh::add_form(),
            Tab::Apps     => apps::add_form(),
            Tab::Settings | Tab::Proxmox => return, // no "add" for these
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
                settings::edit_form(&self.setting_items, idx)
            }
            Tab::Proxmox => return, // no edit for containers
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
            Tab::Settings | Tab::Proxmox => {} // no delete for these
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
                settings::apply_form(&mut self.config, &form);
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
                let next = (self.tab.index() + 1) % 4;
                self.tab = Tab::from_index(next);
                self.status_msg = None;
            }
            KeyCode::Char('1') => { self.tab = Tab::Ssh;        self.status_msg = None; }
            KeyCode::Char('2') => { self.tab = Tab::Apps;       self.status_msg = None; }
            KeyCode::Char('3') => { self.tab = Tab::Settings;   self.status_msg = None; }
            KeyCode::Char('4') => { self.tab = Tab::Proxmox; self.status_msg = None; }
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up   | KeyCode::Char('k') => self.move_up(),
            KeyCode::Char('a') if self.tab != Tab::Proxmox => self.start_add(),
            KeyCode::Char('e') if self.tab != Tab::Proxmox => self.start_edit(),
            KeyCode::Enter if self.tab != Tab::Proxmox => self.start_edit(),
            KeyCode::Char('d') if self.tab != Tab::Proxmox => self.start_delete(),
            KeyCode::Char('c') if self.tab == Tab::Ssh => return self.connect_ssh(),
            // Proxmox: hierarchical navigation
            KeyCode::Enter | KeyCode::Right if self.tab == Tab::Proxmox => self.pve_enter(),
            KeyCode::Backspace | KeyCode::Left if self.tab == Tab::Proxmox => self.pve_back(),
            KeyCode::Char('c') if self.tab == Tab::Proxmox => return self.pve_console(),
            KeyCode::Char('s') if self.tab == Tab::Proxmox => self.pve_start(),
            KeyCode::Char('x') if self.tab == Tab::Proxmox => self.pve_stop_confirm(),
            KeyCode::Char('r') if self.tab == Tab::Proxmox => self.pve_refresh(),
            _ => {}
        }
        false
    }

    fn handle_confirm_key(&mut self, code: KeyCode) -> bool {
        // When on proxmox tab, confirm is for stop, not delete
        if self.tab == Tab::Proxmox {
            match code {
                KeyCode::Char('y') | KeyCode::Enter => self.pve_stop_execute(),
                KeyCode::Char('n') | KeyCode::Esc   => { self.mode = Mode::Normal; }
                KeyCode::Char('q') => return true,
                _ => {}
            }
            return false;
        }
        match code {
            KeyCode::Char('y') | KeyCode::Enter => self.confirm_delete(),
            KeyCode::Char('n') | KeyCode::Esc   => { self.mode = Mode::Normal; }
            KeyCode::Char('q') => return true,
            _ => {}
        }
        false
    }
}

// ─── rendering ───────────────────────────────────────────────────────────────

fn render(f: &mut ratatui::Frame, app: &mut App) {
    let full = f.area();

    // Outer border block
    let outer = Block::default()
        .title(Span::styled(" tmux-config ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));
    let inner = outer.inner(full);
    f.render_widget(outer, full);

    // Split: tab bar (3) | body (fill) | hint bar (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // Tab bar
    tabs::render_tab_bar(f, app.tab, chunks[0]);

    // Body area (may be split for form overlay)
    render_body(f, app, chunks[1]);

    // Hint / status bar
    render_hint(f, app, chunks[2]);
}

fn render_body(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    match &app.mode {
        Mode::Normal => render_list(f, app, area),
        Mode::Confirming { label, .. } => {
            let label = label.clone();
            render_list(f, app, area);
            render_confirm_overlay(f, &label, area, app.tab == Tab::Proxmox);
        }
        Mode::Editing => {
            // Split body: list (upper) | form (lower, ~10 lines)
            let form_height = {
                let fields = app.form.as_ref().map_or(0, |f| f.fields.len());
                // label + each field + divider + hint
                (fields as u16 + 4).min(area.height.saturating_sub(3))
            };
            let list_height = area.height.saturating_sub(form_height);
            let parts = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(list_height), Constraint::Length(form_height)])
                .split(area);
            render_list(f, app, parts[0]);
            render_form(f, app, parts[1]);
        }
    }
}

fn render_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Proxmox => render_proxmox_list(f, app, area),
        _ => render_standard_list(f, app, area),
    }
}

fn render_standard_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let (title, items, list_view) = match app.tab {
        Tab::Ssh => {
            let items: Vec<ListItem> = app.config.ssh.iter()
                .map(|e| ListItem::new(ssh::display(e)))
                .collect();
            ("SSH Hosts", items, &mut app.ssh)
        }
        Tab::Apps => {
            let items: Vec<ListItem> = app.config.apps.iter()
                .map(|a| ListItem::new(apps::display(a)))
                .collect();
            ("Apps", items, &mut app.apps)
        }
        Tab::Settings => {
            let items: Vec<ListItem> = app.setting_items.iter()
                .map(|s| ListItem::new(format!("{:30}  {}", s.label, s.value)))
                .collect();
            ("Settings", items, &mut app.settings)
        }
        Tab::Proxmox => unreachable!(),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(format!(" {title} "), Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().fg(FG).bg(BG))
        .highlight_style(
            Style::default()
                .fg(BLUE)
                .bg(Color::Rgb(58, 63, 76))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut list_view.state);
}

fn render_proxmox_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let breadcrumb = app.pve_breadcrumb();

    let items: Vec<ListItem> = match app.pve_depth {
        0 => app.pve_servers.iter()
            .map(|s| ListItem::new(proxmox::display_server(s)))
            .collect(),
        1 => app.pve_containers.iter()
            .map(|c| ListItem::new(proxmox::display_container(c)))
            .collect(),
        _ => {
            let mut items: Vec<ListItem> = app.pve_docker.iter()
                .map(|d| ListItem::new(proxmox::display_docker(d)))
                .collect();
            for p in &app.pve_ports {
                items.push(ListItem::new(proxmox::display_port(p)));
            }
            items
        }
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(format!(" {breadcrumb} "), Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().fg(FG).bg(BG))
        .highlight_style(
            Style::default()
                .fg(BLUE)
                .bg(Color::Rgb(58, 63, 76))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.pve_list.state);
}

fn render_form(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let form = match &app.form {
        Some(fm) => fm,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        let cursor = if i == form.focused { "\u{2588}" } else { "" };
        let label_style = if i == form.focused {
            Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SUBTLE)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:20} ", field.label), label_style),
            Span::styled(field.value.clone(), Style::default().fg(FG)),
            Span::styled(cursor, Style::default().fg(BLUE)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [Enter]", Style::default().fg(GREEN)),
        Span::raw(" Save  "),
        Span::styled("[Esc]", Style::default().fg(RED)),
        Span::raw(" Cancel  "),
        Span::styled("[Tab]", Style::default().fg(SUBTLE)),
        Span::raw(" Next field"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" Edit ", Style::default().fg(BLUE)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BLUE))
                .style(Style::default().bg(BG)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn render_confirm_overlay(f: &mut ratatui::Frame, label: &str, area: Rect, is_stop: bool) {
    // Center a small box in the area
    let w = 50u16.min(area.width.saturating_sub(4));
    let h = 5u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect::new(x, y, w, h);

    let action = if is_stop { "Stop" } else { "Delete" };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(format!("  {action} '")),
            Span::styled(label, Style::default().fg(RED)),
            Span::raw("' ?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y]", Style::default().fg(RED)),
            Span::raw(" Yes  "),
            Span::styled("[n/Esc]", Style::default().fg(GREEN)),
            Span::raw(" No"),
        ]),
    ];

    let title = if is_stop { " Confirm Stop " } else { " Confirm Delete " };
    let para = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(title, Style::default().fg(RED)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(RED))
            .style(Style::default().bg(BG)),
    );
    f.render_widget(para, rect);
}

fn render_hint(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let text = if let Some(msg) = &app.status_msg {
        Line::from(Span::styled(msg.clone(), Style::default().fg(GREEN)))
    } else {
        match app.mode {
            Mode::Normal => {
                if app.tab == Tab::Proxmox {
                    let mut spans = Vec::new();
                    if app.pve_depth == 0 {
                        spans.extend([Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" open  ")]);
                    } else if app.pve_depth == 1 {
                        spans.extend([
                            Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" docker  "),
                            Span::styled("[c]", Style::default().fg(GREEN)), Span::raw("onsole  "),
                            Span::styled("[s]", Style::default().fg(GREEN)), Span::raw("tart  "),
                            Span::styled("[x]", Style::default().fg(RED)), Span::raw(" stop  "),
                        ]);
                    }
                    if app.pve_depth > 0 {
                        spans.extend([Span::styled("[←/Esc]", Style::default().fg(SUBTLE)), Span::raw(" back  ")]);
                    }
                    spans.extend([
                        Span::styled("[r]", Style::default().fg(BLUE)), Span::raw("efresh  "),
                        Span::styled("[Tab]", Style::default().fg(SUBTLE)), Span::raw(" tab  "),
                        Span::styled("[q]", Style::default().fg(SUBTLE)), Span::raw("uit"),
                    ]);
                    Line::from(spans)
                } else {
                    let mut spans = vec![
                        Span::styled("[a]", Style::default().fg(BLUE)), Span::raw("dd  "),
                        Span::styled("[e]", Style::default().fg(BLUE)), Span::raw("dit  "),
                        Span::styled("[d]", Style::default().fg(RED)),  Span::raw("elete  "),
                    ];
                    if app.tab == Tab::Ssh {
                        spans.push(Span::styled("[c]", Style::default().fg(GREEN)));
                        spans.push(Span::raw("onnect  "));
                    }
                    spans.push(Span::styled("[Tab]", Style::default().fg(SUBTLE)));
                    spans.push(Span::raw(" switch tab  "));
                    spans.push(Span::styled("[q]", Style::default().fg(SUBTLE)));
                    spans.push(Span::raw("uit"));
                    Line::from(spans)
                }
            }
            Mode::Editing => Line::from(vec![
                Span::styled("[Tab]", Style::default().fg(BLUE)), Span::raw(" next field  "),
                Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" save  "),
                Span::styled("[Esc]", Style::default().fg(RED)), Span::raw(" cancel"),
            ]),
            Mode::Confirming { .. } => Line::from(vec![
                Span::styled("[y]", Style::default().fg(RED)), Span::raw(" confirm  "),
                Span::styled("[n/Esc]", Style::default().fg(GREEN)), Span::raw(" cancel"),
            ]),
        }
    };
    let para = Paragraph::new(text)
        .style(Style::default().fg(FG).bg(BG));
    f.render_widget(para, area);
}

// ─── entry point ─────────────────────────────────────────────────────────────

fn main() -> Result<()> {
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
            render(f, app);
        })?;

        match event::read()? {
            Event::Key(key) => {
                let should_quit = match app.mode {
                    Mode::Normal => app.handle_normal_key(key.code),
                    Mode::Editing => {
                        app.handle_form_key(key.code, key.modifiers);
                        false
                    }
                    Mode::Confirming { .. } => app.handle_confirm_key(key.code),
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
                            else if y >= body_area.y + 1 && y < body_area.y + body_area.height.saturating_sub(1) {
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
