use anyhow::Result;
use crate::{proxmox, seed, config_io};
use config_io::{load_config, save_and_apply};

pub(crate) fn cli_dispatch(args: &[String]) -> Result<()> {
    let cmd = args[0].as_str();
    match cmd {
        "help" | "--help" | "-h" => {
            println!("tmux-topbar — tmux status bar configuration manager");
            println!();
            println!("Usage: tmux-topbar [command]");
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
            println!("  pve-logs <server> <vmid>       Tail container logs (tmux session)");
            println!("  pve-docker-logs <server> <vmid> <name>  Tail docker logs");
            println!("  pve-docker-exec <server> <vmid> <name>  Exec into docker");
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
            // Ensure host key is in known_hosts before adding
            proxmox::ensure_host_key(&host);
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
            let user = entry.user.as_deref().unwrap_or("root"); // LINT_ALLOW: default SSH user when entry.user is omitted
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
                let mode_str = a.mode.as_deref().unwrap_or("기본"); // LINT_ALLOW: display label when AppEntry.mode is omitted
                println!("{} {:<20} [{}]", a.emoji, a.command, mode_str);
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
                fg: "#282c34".into(), bg: "#61afef".into(), mode: None,
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
                let tag = match s.access {
                    proxmox::AccessType::Ssh if proxmox::is_localhost(&s.host) => "local",
                    proxmox::AccessType::Ssh => "ssh",
                    proxmox::AccessType::Api => "api",
                };
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
                let user = e.user.as_deref().unwrap_or("root"); // LINT_ALLOW: default SSH user when entry.user is omitted
                let target = format!("{user}@{}", e.host);
                let session_name = format!("ssh-{}", e.name);

                // Ensure host key before connecting
                if !proxmox::host_key_exists(&e.host) {
                    proxmox::ensure_host_key(&e.host);
                }

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
                let user = e.user.as_deref().unwrap_or("root"); // LINT_ALLOW: default SSH user when entry.user is omitted
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
                        let first_line = err.lines().next().unwrap_or("failed"); // LINT_ALLOW: placeholder when stderr is empty on non-zero exit
                        println!("✗ {first_line}");
                    }
                    Err(e) => println!("✗ {e}"),
                }
            }
            Ok(())
        }

        "pve-logs" => {
            if args.len() < 3 { anyhow::bail!("Usage: pve-logs <server> <vmid>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let vmid: u32 = args[2].parse()?;
            let cts = proxmox::fetch_containers(server);
            let ct = cts.iter().find(|c| c.vmid == vmid)
                .ok_or_else(|| anyhow::anyhow!("Container {vmid} not found"))?;
            let cmd = proxmox::container_logs_cmd(server, ct)
                .ok_or_else(|| anyhow::anyhow!("Logs not available"))?;
            let session = format!("log-{vmid}");
            let has = std::process::Command::new("tmux")
                .args(["has-session", "-t", &format!("={session}")])
                .status().map(|s| s.success()).unwrap_or(false);
            if !has {
                std::process::Command::new("tmux")
                    .args(["new-session", "-d", "-s", &session, &cmd]).status()?;
            }
            std::process::Command::new("tmux")
                .args(["switch-client", "-t", &format!("={session}")]).status()?;
            Ok(())
        }

        "pve-docker-logs" => {
            if args.len() < 4 { anyhow::bail!("Usage: pve-docker-logs <server> <vmid> <container>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let vmid: u32 = args[2].parse()?;
            let container = &args[3];
            let cmd = proxmox::docker_logs_cmd(server, vmid, container);
            let session = format!("dlog-{vmid}-{container}");
            let has = std::process::Command::new("tmux")
                .args(["has-session", "-t", &format!("={session}")])
                .status().map(|s| s.success()).unwrap_or(false);
            if !has {
                std::process::Command::new("tmux")
                    .args(["new-session", "-d", "-s", &session, &cmd]).status()?;
            }
            std::process::Command::new("tmux")
                .args(["switch-client", "-t", &format!("={session}")]).status()?;
            Ok(())
        }

        "pve-docker-exec" => {
            if args.len() < 4 { anyhow::bail!("Usage: pve-docker-exec <server> <vmid> <container>"); }
            let config = load_config()?;
            let servers = proxmox::get_servers(&config);
            let server = servers.iter().find(|s| s.name == args[1])
                .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", args[1]))?;
            let vmid: u32 = args[2].parse()?;
            let container = &args[3];
            let cmd = proxmox::docker_exec_cmd(server, vmid, container);
            let session = format!("dexec-{vmid}-{container}");
            let has = std::process::Command::new("tmux")
                .args(["has-session", "-t", &format!("={session}")])
                .status().map(|s| s.success()).unwrap_or(false);
            if !has {
                std::process::Command::new("tmux")
                    .args(["new-session", "-d", "-s", &session, &cmd]).status()?;
            }
            std::process::Command::new("tmux")
                .args(["switch-client", "-t", &format!("={session}")]).status()?;
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
                anyhow::bail!("{} ({}) is running — stop it first with: tmux-topbar pve-stop {} {}", ct.name, vmid, args[1], vmid);
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
                let user = e.user.as_deref().unwrap_or("root"); // LINT_ALLOW: default SSH user when entry.user is omitted
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
                let user = e.user.as_deref().unwrap_or("root"); // LINT_ALLOW: default SSH user when entry.user is omitted
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
            eprintln!("Run 'tmux-topbar help' for usage");
            std::process::exit(1);
        }
    }
}
