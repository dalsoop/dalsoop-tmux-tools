use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::process::Command;
use tmux_windowbar::config::template::Config;

const SSH_OPTS: [&str; 4] = ["-o", "ConnectTimeout=5", "-o", "BatchMode=yes"];

// ── Data ──

#[derive(Clone, Debug)]
pub struct ProxmoxServer {
    pub name: String,
    pub host: String,
    pub user: String,
}

#[derive(Clone, Debug)]
pub struct Container {
    pub vmid: u32,
    pub name: String,
    pub status: String,
    pub kind: String, // "lxc" or "vm"
}

#[derive(Clone, Debug)]
pub struct DockerContainer {
    pub name: String,
    pub status: String,
    pub ports: String,
}

// ── Config helpers ──

pub fn get_servers(config: &Config) -> Vec<ProxmoxServer> {
    config
        .ssh
        .iter()
        .filter(|e| e.r#type == "proxmox")
        .map(|e| ProxmoxServer {
            name: e.name.clone(),
            host: e.host.clone(),
            user: e.user.clone().unwrap_or_else(|| "root".into()),
        })
        .collect()
}

// ── SSH commands ──

fn ssh_run(user: &str, host: &str, cmd: &str) -> Option<String> {
    let target = format!("{user}@{host}");
    let output = Command::new("ssh")
        .args(&SSH_OPTS)
        .arg(&target)
        .arg(cmd)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

// ── Fetch ──

pub fn fetch_containers(server: &ProxmoxServer) -> Vec<Container> {
    let mut result = Vec::new();

    // LXC
    if let Some(out) = ssh_run(&server.user, &server.host, "pct list 2>/dev/null") {
        for line in out.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 3 {
                result.push(Container {
                    vmid: cols[0].parse().unwrap_or(0),
                    status: cols[1].to_string(),
                    name: cols.last().unwrap_or(&"").to_string(),
                    kind: "lxc".into(),
                });
            }
        }
    }

    // VMs
    if let Some(out) = ssh_run(&server.user, &server.host, "qm list 2>/dev/null") {
        for line in out.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 3 {
                result.push(Container {
                    vmid: cols[0].parse().unwrap_or(0),
                    name: cols[1].to_string(),
                    status: cols[2].to_lowercase(),
                    kind: "vm".into(),
                });
            }
        }
    }

    // Sort: running first, then by vmid
    result.sort_by(|a, b| {
        let ar = a.status == "running";
        let br = b.status == "running";
        br.cmp(&ar).then(a.vmid.cmp(&b.vmid))
    });
    result
}

pub fn fetch_docker(server: &ProxmoxServer, vmid: u32) -> Vec<DockerContainer> {
    let cmd = format!(
        "pct exec {vmid} -- docker ps --format '{{{{.Names}}}}\\t{{{{.Status}}}}\\t{{{{.Ports}}}}' 2>/dev/null"
    );
    let out = match ssh_run(&server.user, &server.host, &cmd) {
        Some(o) => o,
        None => return Vec::new(),
    };

    out.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let parts: Vec<&str> = l.splitn(3, '\t').collect();
            DockerContainer {
                name: parts.first().unwrap_or(&"").to_string(),
                status: parts.get(1).unwrap_or(&"").to_string(),
                ports: parts.get(2).unwrap_or(&"").to_string(),
            }
        })
        .collect()
}

pub fn fetch_ports(server: &ProxmoxServer, vmid: u32) -> Vec<String> {
    let cmd = format!(
        "pct exec {vmid} -- ss -tlnp 2>/dev/null | tail -n+2"
    );
    let out = match ssh_run(&server.user, &server.host, &cmd) {
        Some(o) => o,
        None => return Vec::new(),
    };

    out.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            // Extract Local Address:Port column (4th column)
            let cols: Vec<&str> = l.split_whitespace().collect();
            cols.get(3).unwrap_or(&"").to_string()
        })
        .collect()
}

// ── Actions ──

pub fn start_container(server: &ProxmoxServer, c: &Container) {
    let cmd = if c.kind == "vm" {
        format!("qm start {}", c.vmid)
    } else {
        format!("pct start {}", c.vmid)
    };
    let _ = ssh_run(&server.user, &server.host, &cmd);
}

pub fn stop_container(server: &ProxmoxServer, c: &Container) {
    let cmd = if c.kind == "vm" {
        format!("qm stop {}", c.vmid)
    } else {
        format!("pct stop {}", c.vmid)
    };
    let _ = ssh_run(&server.user, &server.host, &cmd);
}

pub fn console_cmd(server: &ProxmoxServer, c: &Container) -> String {
    let enter = if c.kind == "vm" {
        format!("qm terminal {}", c.vmid)
    } else {
        format!("pct enter {}", c.vmid)
    };
    format!("ssh -t {}@{} {enter}", server.user, server.host)
}

// ── Display ──

pub fn display_server(s: &ProxmoxServer) -> Line<'static> {
    Line::from(vec![
        Span::styled("  🖥️ ", Style::default()),
        Span::styled(
            format!("{:<12}", s.name),
            Style::default().fg(Color::Rgb(97, 175, 239)),
        ),
        Span::styled(
            format!("  {}@{}", s.user, s.host),
            Style::default().fg(Color::Rgb(171, 178, 191)),
        ),
    ])
}

pub fn display_container(c: &Container) -> Line<'static> {
    let (dot, dot_color) = if c.status == "running" {
        ("●", Color::Rgb(152, 195, 121))
    } else {
        ("○", Color::Rgb(224, 108, 117))
    };
    Line::from(vec![
        Span::styled(format!("  {dot} "), Style::default().fg(dot_color)),
        Span::styled(format!("{:>6}", c.vmid), Style::default().fg(Color::Rgb(171, 178, 191))),
        Span::styled(format!("  {:<24}", c.name), Style::default().fg(Color::Rgb(97, 175, 239))),
        Span::styled(
            format!("{:<10}", c.status),
            Style::default().fg(if c.status == "running" {
                Color::Rgb(152, 195, 121)
            } else {
                Color::Rgb(224, 108, 117)
            }),
        ),
        Span::styled(format!("  {}", c.kind), Style::default().fg(Color::Rgb(92, 99, 112))),
    ])
}

pub fn display_docker(d: &DockerContainer) -> Line<'static> {
    let color = if d.status.starts_with("Up") {
        Color::Rgb(152, 195, 121)
    } else {
        Color::Rgb(224, 108, 117)
    };
    Line::from(vec![
        Span::styled("  🐳 ", Style::default()),
        Span::styled(format!("{:<24}", d.name), Style::default().fg(color)),
        Span::styled(
            format!("  {}", d.ports),
            Style::default().fg(Color::Rgb(171, 178, 191)),
        ),
    ])
}

pub fn display_port(p: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  🔌 ", Style::default()),
        Span::styled(p.to_string(), Style::default().fg(Color::Rgb(229, 192, 123))),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmux_windowbar::config::template::SshEntry;

    #[test]
    fn get_servers_filters_proxmox_type() {
        let config = Config {
            ssh: vec![
                SshEntry {
                    name: "pve".into(),
                    host: "1.2.3.4".into(),
                    user: Some("root".into()),
                    emoji: "🖥️".into(),
                    fg: String::new(),
                    bg: String::new(),
                    r#type: "proxmox".into(),
                },
                SshEntry {
                    name: "web".into(),
                    host: "5.6.7.8".into(),
                    user: None,
                    emoji: "🌐".into(),
                    fg: String::new(),
                    bg: String::new(),
                    r#type: "ssh".into(),
                },
            ],
            ..Default::default()
        };
        let servers = get_servers(&config);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "pve");
    }

    #[test]
    fn display_container_running() {
        let c = Container {
            vmid: 101,
            name: "gitlab".into(),
            status: "running".into(),
            kind: "lxc".into(),
        };
        let line = display_container(&c);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(text.contains("101"));
        assert!(text.contains("gitlab"));
    }

    #[test]
    fn console_cmd_lxc() {
        let s = ProxmoxServer {
            name: "pve".into(),
            host: "1.2.3.4".into(),
            user: "root".into(),
        };
        let c = Container {
            vmid: 101,
            name: "test".into(),
            status: "running".into(),
            kind: "lxc".into(),
        };
        assert!(console_cmd(&s, &c).contains("pct enter 101"));
    }

    #[test]
    fn console_cmd_vm() {
        let s = ProxmoxServer {
            name: "pve".into(),
            host: "1.2.3.4".into(),
            user: "root".into(),
        };
        let c = Container {
            vmid: 100,
            name: "win".into(),
            status: "running".into(),
            kind: "vm".into(),
        };
        assert!(console_cmd(&s, &c).contains("qm terminal 100"));
    }
}
