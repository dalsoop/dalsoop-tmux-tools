mod apps;
mod config_io;
mod form;
mod list_view;
mod proxmox;
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
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
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
    SeedBrowse { list: ListView },
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
    pve_detail:     Option<proxmox::DetailInfo>,
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
            pve_detail: None,
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
            _ => self.pve_detail.as_ref().map_or(0, |d|
                d.docker.len() + d.ports.len() + d.snapshots.len() + 10 // header lines
            ),
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
                self.pve_sel_ct = Some(idx);
                self.status_msg = Some("Loading details...".into());
                self.pve_detail = Some(proxmox::fetch_detail(server, ct));
                self.pve_depth = 2;
                self.sync_pve_list();
                let d = self.pve_detail.as_ref().unwrap();
                self.status_msg = Some(format!(
                    "{} docker, {} ports, {} snapshots",
                    d.docker.len(), d.ports.len(), d.snapshots.len()
                ));
            }
            _ => {}
        }
    }

    fn pve_back(&mut self) {
        match self.pve_depth {
            2 => {
                self.pve_depth = 1;
                self.pve_detail = None;
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
                    let ct = &self.pve_containers[ci];
                    self.pve_detail = Some(proxmox::fetch_detail(server, ct));
                    self.sync_pve_list();
                }
            }
            _ => {}
        }
    }

    fn pve_connect(&mut self) -> bool {
        match self.pve_depth {
            0 => self.pve_connect_server(),
            1 => self.pve_connect_container(),
            _ => false,
        }
    }

    /// SSH into the selected Proxmox server itself.
    fn pve_connect_server(&mut self) -> bool {
        let idx = match self.pve_list.selected() { Some(i) => i, None => return false };
        if idx >= self.pve_servers.len() { return false; }
        let server = &self.pve_servers[idx];

        if server.access == proxmox::AccessType::Api {
            self.status_msg = Some("SSH not available for API-only servers".into());
            return false;
        }

        let session_name = format!("ssh-{}", server.name);
        let ssh_target = format!("{}@{}", server.user, server.host);
        let ssh_cmd = format!(
            "while true; do ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=3 {ssh_target}; RC=$?; if [ $RC -eq 0 ]; then break; fi; echo '[연결 끊김 - 5초 후 재접속]'; sleep 5; done"
        );

        let has = std::process::Command::new("tmux")
            .args(["has-session", "-t", &format!("={session_name}")])
            .status().map(|s| s.success()).unwrap_or(false);
        if !has {
            let _ = std::process::Command::new("tmux")
                .args(["new-session", "-d", "-s", &session_name, &ssh_cmd])
                .status();
        }
        let _ = std::process::Command::new("tmux")
            .args(["switch-client", "-t", &format!("={session_name}")])
            .status();
        let _ = std::process::Command::new("tmux-sessionbar")
            .args(["render-status", "left"]).status();
        true
    }

    /// Open console to the selected container.
    fn pve_connect_container(&mut self) -> bool {
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

    /// Install SSH key on a proxmox-api server, then upgrade to SSH type.
    fn pve_install_key(&mut self) {
        if self.pve_depth != 0 { return; }
        let idx = match self.pve_list.selected() { Some(i) => i, None => return };
        if idx >= self.pve_servers.len() { return; }
        let server = &self.pve_servers[idx];

        if server.access == proxmox::AccessType::Ssh {
            self.status_msg = Some(format!("{} already uses SSH", server.name));
            return;
        }

        let password = match &server.password {
            Some(p) => p.clone(),
            None => {
                self.status_msg = Some("No password configured for key install".into());
                return;
            }
        };

        let target = format!("{}@{}", server.user, server.host);
        self.status_msg = Some(format!("Installing SSH key on {}...", server.name));

        // Use sshpass + ssh-copy-id
        let ok = std::process::Command::new("sshpass")
            .args(["-p", &password, "ssh-copy-id",
                   "-o", "StrictHostKeyChecking=accept-new", &target])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !ok {
            self.status_msg = Some("Failed to install SSH key".into());
            return;
        }

        // Verify key auth
        let verify = std::process::Command::new("ssh")
            .args(["-o", "ConnectTimeout=5", "-o", "BatchMode=yes", &target, "hostname"])
            .output()
            .ok()
            .filter(|o| o.status.success());

        if verify.is_none() {
            self.status_msg = Some("Key installed but verification failed".into());
            return;
        }

        // Update config: proxmox-api → proxmox
        let server_name = server.name.clone();
        if let Some(entry) = self.config.ssh.iter_mut().find(|e| e.name == server_name) {
            entry.r#type = "proxmox".into();
        }
        let _ = config_io::save_and_apply(&self.config);

        // Refresh servers
        self.pve_servers = proxmox::get_servers(&self.config);
        self.sync_pve_list();
        self.status_msg = Some(format!("{} upgraded to SSH. Console/Docker now available.", server_name));
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

    fn pve_delete_execute(&mut self) {
        let idx = match &self.mode { Mode::Confirming { idx, .. } => *idx, _ => return };
        if idx >= self.pve_containers.len() { self.mode = Mode::Normal; return; }
        let si = self.pve_sel_server.unwrap_or(0);
        let ct = self.pve_containers[idx].clone();
        self.status_msg = Some(format!("Deleting {} {}...", ct.kind, ct.name));
        if proxmox::delete_ct(&self.pve_servers[si], &ct) {
            self.status_msg = Some(format!("{} deleted", ct.name));
        } else {
            self.status_msg = Some(format!("Failed to delete {}", ct.name));
        }
        self.mode = Mode::Normal;
        self.pve_refresh();
    }

    fn pve_delete_confirm(&mut self) {
        if self.pve_depth != 1 { return; }
        let idx = match self.pve_list.selected() { Some(i) => i, None => return };
        if idx >= self.pve_containers.len() { return; }
        let ct = &self.pve_containers[idx];
        if ct.status == "running" {
            self.status_msg = Some(format!("{} is running — stop first", ct.name));
            return;
        }
        let label = format!("DELETE {} {} (irreversible!)", ct.kind, ct.name);
        self.mode = Mode::Confirming { label, idx };
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
                format!("{sname} > {cname} > Detail")
            }
        }
    }

    fn move_down(&mut self) { self.current_list_mut().move_down(); }
    fn move_up(&mut self)   { self.current_list_mut().move_up(); }

    fn start_add(&mut self) {
        self.form = Some(match self.tab {
            Tab::Ssh      => ssh::add_form(),
            Tab::Apps     => apps::add_form(&self.config),
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
                mode: self.config.window.default_app_mode.clone(),
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
            KeyCode::Char(n @ '1'..='9') => {
                let idx = (n as usize) - ('1' as usize);
                if idx < Tab::titles().len() {
                    self.tab = Tab::from_index(idx);
                    self.status_msg = None;
                }
            }
            KeyCode::Char('4') => { self.tab = Tab::Proxmox; self.status_msg = None; }
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
            let form_height = {
                let fields = app.form.as_ref().map_or(0, |f| f.fields.len());
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
        Mode::SeedBrowse { list } => {
            let items: Vec<ListItem> = seed::SEEDS.iter()
                .map(|s| {
                    let installed = seed::is_installed(s.command);
                    let in_config = app.config.apps.iter().any(|a| a.command == s.command);
                    let method = seed::install_method(s);
                    let status = if in_config {
                        "✓ added".to_string()
                    } else if installed {
                        "✓ ready".to_string()
                    } else if method != "-" {
                        format!("✗ via {method}")
                    } else {
                        "✗ n/a".to_string()
                    };
                    let style = if in_config {
                        Style::default().fg(SUBTLE)
                    } else if installed {
                        Style::default().fg(GREEN)
                    } else if method != "-" {
                        Style::default().fg(FG)
                    } else {
                        Style::default().fg(RED)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("  {} {:<14} ", s.emoji, s.command), style),
                        Span::styled(format!("{:<12} ", status), if installed || in_config { Style::default().fg(GREEN) } else if method != "-" { Style::default().fg(BLUE) } else { Style::default().fg(RED) }),
                        Span::styled(s.description, Style::default().fg(SUBTLE)),
                    ]))
                })
                .collect();

            let mut state = list.state.clone();
            let widget = List::new(items)
                .block(
                    Block::default()
                        .title(Span::styled(" Install Apps ", Style::default().fg(GREEN)))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(BLUE))
                        .style(Style::default().bg(BG)),
                )
                .style(Style::default().fg(FG).bg(BG))
                .highlight_style(Style::default().fg(BLUE).bg(Color::Rgb(58, 63, 76)).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            f.render_stateful_widget(widget, area, &mut state);
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
            if let Some(detail) = &app.pve_detail {
                proxmox::display_detail(detail).into_iter()
                    .map(ListItem::new)
                    .collect()
            } else {
                vec![ListItem::new("  (no data)")]
            }
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
                        spans.extend([
                            Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" open  "),
                            Span::styled("[c]", Style::default().fg(GREEN)), Span::raw("onnect  "),
                            Span::styled("[k]", Style::default().fg(BLUE)), Span::raw("ey install  "),
                        ]);
                    } else if app.pve_depth == 1 {
                        spans.extend([
                            Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" docker  "),
                            Span::styled("[c]", Style::default().fg(GREEN)), Span::raw("onsole  "),
                            Span::styled("[s]", Style::default().fg(GREEN)), Span::raw("tart  "),
                            Span::styled("[x]", Style::default().fg(RED)), Span::raw(" stop  "),
                            Span::styled("[D]", Style::default().fg(RED)), Span::raw("elete  "),
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
                    if app.tab == Tab::Apps {
                        spans.push(Span::styled("[i]", Style::default().fg(GREEN)));
                        spans.push(Span::raw("nstall  "));
                    }
                    spans.push(Span::styled("[Tab]", Style::default().fg(SUBTLE)));
                    spans.push(Span::raw(" switch tab  "));
                    spans.push(Span::styled("[q]", Style::default().fg(SUBTLE)));
                    spans.push(Span::raw("uit"));
                    Line::from(spans)
                }
            }
            Mode::SeedBrowse { .. } => Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" install & add  "),
                Span::styled("[Esc]", Style::default().fg(RED)), Span::raw(" back"),
            ]),
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
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        return cli_dispatch(&args[1..]);
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

// ─── CLI mode ────────────────────────────────────────────────────────────────

fn cli_dispatch(args: &[String]) -> Result<()> {
    let cmd = args[0].as_str();
    match cmd {
        "help" | "--help" | "-h" => {
            println!("tmux-config — tmux status bar configuration manager");
            println!();
            println!("Usage: tmux-config [command]");
            println!();
            println!("  (no args)              Open TUI");
            println!();
            println!("  ssh-list               List SSH hosts");
            println!("  ssh-add <name> <host> [user] [type]");
            println!("                         Add SSH host (type: ssh|proxmox|proxmox-api)");
            println!("  ssh-rm <name>          Remove SSH host");
            println!("  ssh-connect <name>     Connect to SSH host (tmux session)");
            println!();
            println!("  app-list               List apps");
            println!("  app-add <cmd> [emoji]  Add app");
            println!("  app-rm <cmd>           Remove app");
            println!();
            println!("  pve-list               List Proxmox servers");
            println!("  pve-ct <server>        List containers on server");
            println!("  pve-start <server> <vmid>  Start container");
            println!("  pve-stop <server> <vmid>   Stop container");
            println!("  pve-key <server>       Install SSH key on API server");
            println!("  pve-create <server> <name> <template> [mem] [cores] [disk]");
            println!("                         Create LXC (default: 512MB, 1 core, 8GB)");
            println!("  pve-clone <server> <vmid> <name>");
            println!("                         Clone container/VM");
            println!("  pve-delete <server> <vmid>  Delete container/VM (must be stopped)");
            println!("  pve-templates <server> List available templates");
            println!();
            println!("  connect-all            Open tmux sessions for all SSH hosts");
            println!("  exec-all <command>     Run command on all SSH hosts");
            println!("  status                 Check connectivity of all hosts");
            println!("  deploy <app> [host]    Install app on all hosts (or specific host)");
            println!("                         Apps: claude, codex, gemini, htop, btop, lazygit...");
            Ok(())
        }

        "ssh-list" => {
            let config = load_config()?;
            for e in &config.ssh {
                let user = e.user.as_deref().unwrap_or("-");
                println!("{:<15} {}@{:<20} [{}]", e.name, user, e.host, e.r#type);
            }
            Ok(())
        }

        "ssh-add" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: ssh-add <name> <host> [user] [type]");
            }
            let mut config = load_config()?;
            let name = args[1].clone();
            let host = args[2].clone();
            let user = args.get(3).filter(|s| !s.is_empty()).cloned();
            let entry_type = args.get(4).cloned().unwrap_or_else(|| "ssh".into());
            config.ssh.push(tmux_windowbar::config::template::SshEntry {
                name: name.clone(), host, user,
                emoji: "\u{1f5a5}\u{fe0f}".into(),
                fg: "#abb2bf".into(), bg: "#3e4452".into(),
                r#type: entry_type, password: None, port: None,
            });
            save_and_apply(&config)?;
            println!("Added '{name}'");
            Ok(())
        }

        "ssh-rm" => {
            if args.len() < 2 { anyhow::bail!("Usage: ssh-rm <name>"); }
            let mut config = load_config()?;
            let name = &args[1];
            let before = config.ssh.len();
            config.ssh.retain(|e| e.name != *name);
            if config.ssh.len() == before {
                anyhow::bail!("SSH host '{name}' not found");
            }
            save_and_apply(&config)?;
            println!("Removed '{name}'");
            Ok(())
        }

        "ssh-connect" => {
            if args.len() < 2 { anyhow::bail!("Usage: ssh-connect <name>"); }
            let config = load_config()?;
            let name = &args[1];
            let entry = config.ssh.iter().find(|e| e.name == *name)
                .ok_or_else(|| anyhow::anyhow!("SSH host '{name}' not found"))?;
            let user = entry.user.as_deref().unwrap_or("root");
            let target = format!("{user}@{}", entry.host);
            let session_name = format!("ssh-{name}");
            let ssh_cmd = format!(
                "while true; do ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=3 {target}; RC=$?; if [ $RC -eq 0 ]; then break; fi; echo '[연결 끊김 - 5초 후 재접속]'; sleep 5; done"
            );
            let has = std::process::Command::new("tmux")
                .args(["has-session", "-t", &format!("={session_name}")])
                .status().map(|s| s.success()).unwrap_or(false);
            if !has {
                std::process::Command::new("tmux")
                    .args(["new-session", "-d", "-s", &session_name, &ssh_cmd])
                    .status()?;
            }
            std::process::Command::new("tmux")
                .args(["switch-client", "-t", &format!("={session_name}")])
                .status()?;
            Ok(())
        }

        "app-list" => {
            let config = load_config()?;
            for a in &config.apps {
                println!("{} {:<20} [{}]", a.emoji, a.command, a.mode);
            }
            Ok(())
        }

        "app-add" => {
            if args.len() < 2 { anyhow::bail!("Usage: app-add <command> [emoji]"); }
            let mut config = load_config()?;
            let command = args[1].clone();
            let emoji = args.get(2).cloned().unwrap_or_else(|| "🔧".into());
            config.apps.push(tmux_windowbar::config::template::AppEntry {
                emoji, command: command.clone(),
                fg: "#282c34".into(), bg: "#61afef".into(), mode: "window".into(),
            });
            save_and_apply(&config)?;
            println!("Added '{command}'");
            Ok(())
        }

        "app-rm" => {
            if args.len() < 2 { anyhow::bail!("Usage: app-rm <command>"); }
            let mut config = load_config()?;
            let cmd = &args[1];
            let before = config.apps.len();
            config.apps.retain(|a| a.command != *cmd);
            if config.apps.len() == before {
                anyhow::bail!("App '{cmd}' not found");
            }
            save_and_apply(&config)?;
            println!("Removed '{cmd}'");
            Ok(())
        }

        "pve-list" => {
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            for s in &servers {
                let tag = match s.access { proxmox::AccessType::Ssh => "ssh", proxmox::AccessType::Api => "api" };
                println!("{:<15} {}@{:<20} [{}]", s.name, s.user, s.host, tag);
            }
            Ok(())
        }

        "pve-ct" => {
            if args.len() < 2 { anyhow::bail!("Usage: pve-ct <server-name>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let cts = proxmox::fetch_containers(server);
            for c in &cts {
                println!("{:>6} {:<24} {:<10} {}", c.vmid, c.name, c.status, c.kind);
            }
            Ok(())
        }

        "pve-start" => {
            if args.len() < 3 { anyhow::bail!("Usage: pve-start <server> <vmid>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let vmid: u32 = args[2].parse()?;
            let cts = proxmox::fetch_containers(server);
            let ct = cts.iter().find(|c| c.vmid == vmid)
                .ok_or_else(|| anyhow::anyhow!("Container {vmid} not found"))?;
            proxmox::start_container(server, ct);
            println!("Started {vmid}");
            Ok(())
        }

        "pve-stop" => {
            if args.len() < 3 { anyhow::bail!("Usage: pve-stop <server> <vmid>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let vmid: u32 = args[2].parse()?;
            let cts = proxmox::fetch_containers(server);
            let ct = cts.iter().find(|c| c.vmid == vmid)
                .ok_or_else(|| anyhow::anyhow!("Container {vmid} not found"))?;
            proxmox::stop_container(server, ct);
            println!("Stopped {vmid}");
            Ok(())
        }

        "pve-key" => {
            if args.len() < 2 { anyhow::bail!("Usage: pve-key <server-name>"); }
            let mut config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            if server.access == proxmox::AccessType::Ssh {
                println!("{} already uses SSH", server.name);
                return Ok(());
            }
            let password = server.password.as_deref()
                .ok_or_else(|| anyhow::anyhow!("No password configured"))?;
            let target = format!("{}@{}", server.user, server.host);
            let ok = std::process::Command::new("sshpass")
                .args(["-p", password, "ssh-copy-id", "-o", "StrictHostKeyChecking=accept-new", &target])
                .status().map(|s| s.success()).unwrap_or(false);
            if !ok { anyhow::bail!("Failed to install SSH key"); }
            // Upgrade config
            let name = server.name.clone();
            if let Some(entry) = config.ssh.iter_mut().find(|e| e.name == name) {
                entry.r#type = "proxmox".into();
            }
            save_and_apply(&config)?;
            println!("{name} upgraded to SSH");
            Ok(())
        }

        "connect-all" => {
            let config = load_config()?;
            for e in &config.ssh {
                if e.r#type == "proxmox-api" { continue; } // skip API-only
                let user = e.user.as_deref().unwrap_or("root");
                let target = format!("{user}@{}", e.host);
                let session_name = format!("ssh-{}", e.name);

                // Check connectivity first
                let reachable = std::process::Command::new("ssh")
                    .args(["-o", "ConnectTimeout=3", "-o", "BatchMode=yes", &target, "echo ok"])
                    .output().map(|o| o.status.success()).unwrap_or(false);
                if !reachable {
                    println!("  ✗ {:<15} unreachable", e.name);
                    continue;
                }

                let has = std::process::Command::new("tmux")
                    .args(["has-session", "-t", &format!("={session_name}")])
                    .status().map(|s| s.success()).unwrap_or(false);
                if has {
                    println!("  ✓ {:<15} already connected", e.name);
                    continue;
                }

                let ssh_cmd = format!(
                    "while true; do ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=3 {target}; RC=$?; if [ $RC -eq 0 ]; then break; fi; echo '[연결 끊김 - 5초 후 재접속]'; sleep 5; done"
                );
                let ok = std::process::Command::new("tmux")
                    .args(["new-session", "-d", "-s", &session_name, &ssh_cmd])
                    .status().map(|s| s.success()).unwrap_or(false);
                if ok {
                    println!("  ✓ {:<15} connected", e.name);
                } else {
                    println!("  ✗ {:<15} failed to create session", e.name);
                }
            }
            // Refresh status bar
            let _ = std::process::Command::new("tmux-sessionbar")
                .args(["render-status", "left"]).status();
            Ok(())
        }

        "exec-all" => {
            if args.len() < 2 { anyhow::bail!("Usage: exec-all <command>"); }
            let remote_cmd = args[1..].join(" ");
            let config = load_config()?;
            for e in &config.ssh {
                if e.r#type == "proxmox-api" { continue; }
                let user = e.user.as_deref().unwrap_or("root");
                let target = format!("{user}@{}", e.host);
                print!("  {:<15} ", e.name);
                let output = std::process::Command::new("ssh")
                    .args(["-o", "ConnectTimeout=5", "-o", "BatchMode=yes", &target, &remote_cmd])
                    .output();
                match output {
                    Ok(o) if o.status.success() => {
                        let out = String::from_utf8_lossy(&o.stdout);
                        let first_line = out.lines().next().unwrap_or("");
                        println!("✓ {first_line}");
                    }
                    Ok(o) => {
                        let err = String::from_utf8_lossy(&o.stderr);
                        let first_line = err.lines().next().unwrap_or("failed");
                        println!("✗ {first_line}");
                    }
                    Err(e) => println!("✗ {e}"),
                }
            }
            Ok(())
        }

        "pve-templates" => {
            if args.len() < 2 { anyhow::bail!("Usage: pve-templates <server>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            println!("Local templates:");
            for t in proxmox::list_local_templates(server) {
                println!("  {t}");
            }
            println!("\nAvailable to download:");
            for t in proxmox::list_templates(server) {
                println!("  {t}");
            }
            Ok(())
        }

        "pve-create" => {
            if args.len() < 4 { anyhow::bail!("Usage: pve-create <server> <name> <template> [mem] [cores] [disk]"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let hostname = &args[2];
            let template = &args[3];
            let memory: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(512);
            let cores: u32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);
            let disk: u32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(8);

            let vmid = proxmox::next_vmid(server)
                .ok_or_else(|| anyhow::anyhow!("Failed to get next VMID"))?;
            println!("Creating LXC {vmid} ({hostname}) on {}...", server.name);
            println!("  Template: {template}");
            println!("  Memory: {memory}MB, Cores: {cores}, Disk: {disk}GB");

            if proxmox::create_lxc(server, vmid, hostname, template, memory, cores, disk, "changeme") {
                println!("✓ Created {vmid} ({hostname}) — default password: changeme");
            } else {
                println!("✗ Failed to create container");
            }
            Ok(())
        }

        "pve-clone" => {
            if args.len() < 4 { anyhow::bail!("Usage: pve-clone <server> <vmid> <name>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let src_vmid: u32 = args[2].parse()?;
            let hostname = &args[3];

            let cts = proxmox::fetch_containers(server);
            let ct = cts.iter().find(|c| c.vmid == src_vmid)
                .ok_or_else(|| anyhow::anyhow!("Container {src_vmid} not found"))?;

            let new_vmid = proxmox::next_vmid(server)
                .ok_or_else(|| anyhow::anyhow!("Failed to get next VMID"))?;

            println!("Cloning {} ({}) → {new_vmid} ({hostname})...", ct.vmid, ct.name);
            if proxmox::clone_ct(server, src_vmid, new_vmid, hostname, &ct.kind) {
                println!("✓ Cloned to {new_vmid} ({hostname})");
            } else {
                println!("✗ Clone failed");
            }
            Ok(())
        }

        "pve-delete" => {
            if args.len() < 3 { anyhow::bail!("Usage: pve-delete <server> <vmid>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let vmid: u32 = args[2].parse()?;
            let cts = proxmox::fetch_containers(server);
            let ct = cts.iter().find(|c| c.vmid == vmid)
                .ok_or_else(|| anyhow::anyhow!("Container {vmid} not found"))?;

            if ct.status == "running" {
                anyhow::bail!("{} ({}) is running — stop it first with: tmux-config pve-stop {} {}", ct.name, vmid, args[1], vmid);
            }

            println!("Deleting {} {} ({})...", ct.kind, vmid, ct.name);
            if proxmox::delete_ct(server, ct) {
                println!("✓ Deleted {vmid} ({name})", name = ct.name);
            } else {
                println!("✗ Delete failed");
            }
            Ok(())
        }

        "deploy" => {
            if args.len() < 2 { anyhow::bail!("Usage: deploy <app> [host]\nApps: claude, codex, gemini, htop, btop, lazygit, lazydocker, opencode"); }
            let app_name = &args[1];
            let target_host = args.get(2).map(|s| s.as_str());

            let app = seed::find(app_name)
                .ok_or_else(|| anyhow::anyhow!("Unknown app: {app_name}. Available: claude, codex, gemini, htop, btop, lazygit, lazydocker, opencode"))?;
            let script = seed::remote_install_script(app)
                .ok_or_else(|| anyhow::anyhow!("No remote install method for {app_name}"))?;

            let config = load_config()?;
            let hosts: Vec<_> = config.ssh.iter()
                .filter(|e| e.r#type != "proxmox-api")
                .filter(|e| target_host.is_none() || target_host == Some(e.name.as_str()))
                .collect();

            if hosts.is_empty() {
                anyhow::bail!("No matching hosts found");
            }

            for e in &hosts {
                let user = e.user.as_deref().unwrap_or("root");
                let target = format!("{user}@{}", e.host);
                println!("── {} ({}) ──", e.name, target);

                // Check if already installed
                let already = std::process::Command::new("ssh")
                    .args(["-o", "ConnectTimeout=5", "-o", "BatchMode=yes", &target,
                           &format!("command -v {} >/dev/null 2>&1 && echo yes || echo no", app.command)])
                    .output().ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

                if already.as_deref() == Some("yes") {
                    println!("  ✓ {} already installed", app.command);
                    continue;
                }

                // Run install script
                let result = std::process::Command::new("ssh")
                    .args(["-o", "ConnectTimeout=30", "-o", "BatchMode=yes", &target, &script])
                    .status();

                match result {
                    Ok(s) if s.success() => println!("  ✓ {} installed", app.command),
                    Ok(_) => println!("  ✗ {} install failed", app.command),
                    Err(e) => println!("  ✗ connection error: {e}"),
                }
            }
            Ok(())
        }

        "status" => {
            let config = load_config()?;
            for e in &config.ssh {
                let user = e.user.as_deref().unwrap_or("root");
                let target = format!("{user}@{}", e.host);
                print!("  {:<15} {:<25} [{:<10}] ", e.name, target, e.r#type);

                if e.r#type == "proxmox-api" {
                    // Check API
                    let url = format!("https://{}:{}/api2/json/version",
                        e.host, e.port.unwrap_or(8006));
                    let ok = std::process::Command::new("curl")
                        .args(["-sk", "--connect-timeout", "3", &url])
                        .output().map(|o| o.status.success() && !o.stdout.is_empty())
                        .unwrap_or(false);
                    println!("{}", if ok { "✓ api" } else { "✗ unreachable" });
                } else {
                    // Check SSH
                    let ok = std::process::Command::new("ssh")
                        .args(["-o", "ConnectTimeout=3", "-o", "BatchMode=yes", &target, "echo ok"])
                        .output().map(|o| o.status.success()).unwrap_or(false);

                    // Check tmux session
                    let session = format!("ssh-{}", e.name);
                    let has_session = std::process::Command::new("tmux")
                        .args(["has-session", "-t", &format!("={session}")])
                        .status().map(|s| s.success()).unwrap_or(false);

                    match (ok, has_session) {
                        (true, true)   => println!("✓ ssh + session"),
                        (true, false)  => println!("✓ ssh (no session)"),
                        (false, true)  => println!("✗ unreachable (stale session)"),
                        (false, false) => println!("✗ unreachable"),
                    }
                }
            }
            Ok(())
        }

        _ => {
            eprintln!("Unknown command: {cmd}");
            eprintln!("Run 'tmux-config help' for usage");
            std::process::exit(1);
        }
    }
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
