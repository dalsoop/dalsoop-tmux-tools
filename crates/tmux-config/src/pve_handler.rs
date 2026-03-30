use crate::{App, Mode, proxmox, config_io};

impl App {
    // Proxmox drill-down
    pub(crate) fn pve_enter(&mut self) {
        match self.pve_depth {
            0 => {
                // Enter server → fetch containers
                let idx = match self.pve_list.selected() { Some(i) => i, None => return };
                if idx >= self.pve_servers.len() { return; }
                self.pve_sel_server = Some(idx);
                self.status_msg = Some("Loading host info + containers...".into());
                self.pve_host_info = proxmox::fetch_host_info(&self.pve_servers[idx]);
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

    pub(crate) fn pve_back(&mut self) {
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
                self.pve_host_info = None;
                self.sync_pve_list();
                self.status_msg = None;
            }
            _ => {}
        }
    }

    pub(crate) fn pve_refresh(&mut self) {
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

    pub(crate) fn pve_connect(&mut self) -> bool {
        match self.pve_depth {
            0 => self.pve_connect_server(),
            1 => self.pve_connect_container(),
            _ => false,
        }
    }

    /// SSH into the selected Proxmox server itself.
    pub(crate) fn pve_connect_server(&mut self) -> bool {
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
    pub(crate) fn pve_connect_container(&mut self) -> bool {
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
    pub(crate) fn pve_install_key(&mut self) {
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

    pub(crate) fn pve_start(&mut self) {
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

    /// Get the currently selected docker container name (at depth 2).
    pub(crate) fn pve_selected_docker(&self) -> Option<String> {
        let detail = self.pve_detail.as_ref()?;
        let sel = self.pve_list.selected()?;
        // The detail view has header lines mixed in — need to map selection to docker index.
        // Count through display_detail lines to find which docker container is selected.
        let lines = proxmox::display_detail(detail);
        if sel >= lines.len() { return None; }
        // Find the docker container: look for lines with docker container names
        let mut docker_idx: Option<usize> = None;
        let mut current_docker = 0usize;
        let mut in_docker_section = false;
        for (i, _line) in lines.iter().enumerate() {
            let text: String = _line.spans.iter().map(|s| s.content.to_string()).collect();
            if text.contains("🐳 Docker") { in_docker_section = true; continue; }
            if text.contains("🔌 Listening") { in_docker_section = false; }
            if in_docker_section && !text.trim().starts_with("↳") && !text.contains("(none") && !text.trim().is_empty() {
                if i == sel {
                    docker_idx = Some(current_docker);
                    break;
                }
                current_docker += 1;
            }
        }
        docker_idx.and_then(|i| detail.docker.get(i).map(|d| d.name.clone()))
    }

    /// Open container system logs in tmux session.
    pub(crate) fn pve_container_logs(&mut self) -> bool {
        if self.pve_depth != 1 { return false; }
        let idx = match self.pve_list.selected() { Some(i) => i, None => return false };
        if idx >= self.pve_containers.len() { return false; }
        let si = self.pve_sel_server.unwrap_or(0);
        let server = &self.pve_servers[si];
        let ct = &self.pve_containers[idx];
        let cmd = match proxmox::container_logs_cmd(server, ct) {
            Some(c) => c,
            None => { self.status_msg = Some("Logs not available".into()); return false; }
        };
        let session = format!("log-{}", ct.vmid);
        self.open_tmux_session(&session, &cmd)
    }

    /// Open docker container logs in tmux session.
    pub(crate) fn pve_docker_logs(&mut self) -> bool {
        let docker_name = match self.pve_selected_docker() {
            Some(n) => n,
            None => { self.status_msg = Some("Select a docker container".into()); return false; }
        };
        let si = self.pve_sel_server.unwrap_or(0);
        let ci = self.pve_sel_ct.unwrap_or(0);
        if si >= self.pve_servers.len() || ci >= self.pve_containers.len() { return false; }
        let server = &self.pve_servers[si];
        let vmid = self.pve_containers[ci].vmid;
        let cmd = proxmox::docker_logs_cmd(server, vmid, &docker_name);
        let session = format!("dlog-{}-{}", vmid, docker_name);
        self.open_tmux_session(&session, &cmd)
    }

    /// Exec into docker container.
    pub(crate) fn pve_docker_exec(&mut self) -> bool {
        let docker_name = match self.pve_selected_docker() {
            Some(n) => n,
            None => { self.status_msg = Some("Select a docker container".into()); return false; }
        };
        let si = self.pve_sel_server.unwrap_or(0);
        let ci = self.pve_sel_ct.unwrap_or(0);
        if si >= self.pve_servers.len() || ci >= self.pve_containers.len() { return false; }
        let server = &self.pve_servers[si];
        let vmid = self.pve_containers[ci].vmid;
        let cmd = proxmox::docker_exec_cmd(server, vmid, &docker_name);
        let session = format!("dexec-{}-{}", vmid, docker_name);
        self.open_tmux_session(&session, &cmd)
    }

    /// Docker start/stop/restart.
    pub(crate) fn pve_docker_action(&mut self, action: &str) {
        let docker_name = match self.pve_selected_docker() {
            Some(n) => n,
            None => { self.status_msg = Some("Select a docker container".into()); return; }
        };
        let si = self.pve_sel_server.unwrap_or(0);
        let ci = self.pve_sel_ct.unwrap_or(0);
        if si >= self.pve_servers.len() || ci >= self.pve_containers.len() { return; }
        let server = &self.pve_servers[si];
        let vmid = self.pve_containers[ci].vmid;
        self.status_msg = Some(format!("{} {}...", action, docker_name));
        let ok = match action {
            "start" => proxmox::docker_start(server, vmid, &docker_name),
            "stop" => proxmox::docker_stop(server, vmid, &docker_name),
            "restart" => proxmox::docker_restart(server, vmid, &docker_name),
            _ => false,
        };
        if ok {
            self.status_msg = Some(format!("{} {}", action, docker_name));
            self.pve_refresh();
        } else {
            self.status_msg = Some(format!("Failed to {} {}", action, docker_name));
        }
    }

    /// Helper: open a tmux session with a command and switch to it.
    pub(crate) fn open_tmux_session(&self, name: &str, cmd: &str) -> bool {
        let has = std::process::Command::new("tmux")
            .args(["has-session", "-t", &format!("={name}")])
            .status().map(|s| s.success()).unwrap_or(false);
        if !has {
            let _ = std::process::Command::new("tmux")
                .args(["new-session", "-d", "-s", name, cmd])
                .status();
        }
        let _ = std::process::Command::new("tmux")
            .args(["switch-client", "-t", &format!("={name}")])
            .status();
        true
    }

    pub(crate) fn pve_delete_execute(&mut self) {
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

    pub(crate) fn pve_delete_confirm(&mut self) {
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

    pub(crate) fn pve_stop_confirm(&mut self) {
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

    pub(crate) fn pve_stop_execute(&mut self) {
        let idx = match &self.mode { Mode::Confirming { idx, .. } => *idx, _ => return };
        if idx >= self.pve_containers.len() { self.mode = Mode::Normal; return; }
        let si = self.pve_sel_server.unwrap_or(0);
        let ct = self.pve_containers[idx].clone();
        self.status_msg = Some(format!("Stopping {}...", ct.name));
        proxmox::stop_container(&self.pve_servers[si], &ct);
        self.mode = Mode::Normal;
        self.pve_refresh();
    }

    pub(crate) fn pve_breadcrumb(&self) -> String {
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
}
