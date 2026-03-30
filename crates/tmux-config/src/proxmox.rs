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
    pub access: AccessType,
    pub password: Option<String>,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AccessType {
    Ssh,
    Api,
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
        .filter(|e| e.r#type == "proxmox" || e.r#type == "proxmox-api")
        .map(|e| ProxmoxServer {
            name: e.name.clone(),
            host: e.host.clone(),
            user: e.user.clone().unwrap_or_else(|| "root".into()),
            access: if e.r#type == "proxmox-api" { AccessType::Api } else { AccessType::Ssh },
            password: e.password.clone(),
            port: e.port.unwrap_or(8006),
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

// ── API helpers ──

fn api_get_ticket(server: &ProxmoxServer) -> Option<String> {
    let url = format!("https://{}:{}/api2/json/access/ticket", server.host, server.port);
    let user = format!("{}@pam", server.user);
    let password = server.password.as_deref().unwrap_or("");
    let output = Command::new("curl")
        .args(["-sk", "--connect-timeout", "5", "-d",
               &format!("username={user}&password={password}"), &url])
        .output().ok()?;
    if !output.status.success() { return None; }
    let text = String::from_utf8_lossy(&output.stdout);
    // Extract ticket from JSON
    let ticket_start = text.find("\"ticket\":\"")?;
    let rest = &text[ticket_start + 10..];
    let ticket_end = rest.find('"')?;
    Some(rest[..ticket_end].to_string())
}

fn api_get(server: &ProxmoxServer, ticket: &str, path: &str) -> Option<String> {
    let url = format!("https://{}:{}/api2/json{path}", server.host, server.port);
    let cookie = format!("PVEAuthCookie={ticket}");
    let output = Command::new("curl")
        .args(["-sk", "--connect-timeout", "5", "-b", &cookie, &url])
        .output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

fn api_post(server: &ProxmoxServer, ticket: &str, csrf: &str, path: &str) -> bool {
    let url = format!("https://{}:{}/api2/json{path}", server.host, server.port);
    let cookie = format!("PVEAuthCookie={ticket}");
    Command::new("curl")
        .args(["-sk", "--connect-timeout", "10", "-X", "POST",
               "-b", &cookie, "-H", &format!("CSRFPreventionToken: {csrf}"), &url])
        .status().map(|s| s.success()).unwrap_or(false)
}

fn api_get_ticket_and_csrf(server: &ProxmoxServer) -> Option<(String, String)> {
    let url = format!("https://{}:{}/api2/json/access/ticket", server.host, server.port);
    let user = format!("{}@pam", server.user);
    let password = server.password.as_deref().unwrap_or("");
    let output = Command::new("curl")
        .args(["-sk", "--connect-timeout", "5", "-d",
               &format!("username={user}&password={password}"), &url])
        .output().ok()?;
    if !output.status.success() { return None; }
    let text = String::from_utf8_lossy(&output.stdout);

    let ticket = {
        let start = text.find("\"ticket\":\"")?;
        let rest = &text[start + 10..];
        let end = rest.find('"')?;
        rest[..end].to_string()
    };
    let csrf = {
        let start = text.find("\"CSRFPreventionToken\":\"")?;
        let rest = &text[start + 23..];
        let end = rest.find('"')?;
        rest[..end].to_string()
    };
    Some((ticket, csrf))
}

fn api_get_node(server: &ProxmoxServer, ticket: &str) -> Option<String> {
    let text = api_get(server, ticket, "/nodes")?;
    let start = text.find("\"node\":\"")?;
    let rest = &text[start + 8..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn api_fetch_containers(server: &ProxmoxServer) -> Vec<Container> {
    let ticket = match api_get_ticket(server) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let node = match api_get_node(server, &ticket) {
        Some(n) => n,
        None => return Vec::new(),
    };

    let mut result = Vec::new();

    // LXC
    if let Some(text) = api_get(server, &ticket, &format!("/nodes/{node}/lxc")) {
        // Simple JSON parsing without serde_json
        for entry in text.split("\"vmid\":").skip(1) {
            let vmid: u32 = entry.split(|c: char| !c.is_ascii_digit()).next()
                .unwrap_or("0").parse().unwrap_or(0);
            let name = extract_json_str(entry, "name").unwrap_or_default();
            let status = extract_json_str(entry, "status").unwrap_or_default();
            if vmid > 0 {
                result.push(Container { vmid, name, status, kind: "lxc".into() });
            }
        }
    }

    // VMs
    if let Some(text) = api_get(server, &ticket, &format!("/nodes/{node}/qemu")) {
        for entry in text.split("\"vmid\":").skip(1) {
            let vmid: u32 = entry.split(|c: char| !c.is_ascii_digit()).next()
                .unwrap_or("0").parse().unwrap_or(0);
            let name = extract_json_str(entry, "name").unwrap_or_default();
            let status = extract_json_str(entry, "status").unwrap_or_default();
            if vmid > 0 {
                result.push(Container { vmid, name, status, kind: "vm".into() });
            }
        }
    }

    result
}

fn extract_json_str(text: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":\"");
    let start = text.find(&pattern)?;
    let rest = &text[start + pattern.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

// ── Fetch (dispatch) ──

pub fn fetch_containers(server: &ProxmoxServer) -> Vec<Container> {
    let mut result = match server.access {
        AccessType::Api => api_fetch_containers(server),
        AccessType::Ssh => ssh_fetch_containers(server),
    };

    // Sort: running first, then by vmid
    result.sort_by(|a, b| {
        let ar = a.status == "running";
        let br = b.status == "running";
        br.cmp(&ar).then(a.vmid.cmp(&b.vmid))
    });
    result
}

fn ssh_fetch_containers(server: &ProxmoxServer) -> Vec<Container> {
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

    result
}

pub fn fetch_docker(server: &ProxmoxServer, vmid: u32) -> Vec<DockerContainer> {
    if server.access == AccessType::Api { return Vec::new(); } // Docker inspection requires SSH
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
    if server.access == AccessType::Api { return Vec::new(); } // Port inspection requires SSH
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
    match server.access {
        AccessType::Ssh => {
            let cmd = if c.kind == "vm" {
                format!("qm start {}", c.vmid)
            } else {
                format!("pct start {}", c.vmid)
            };
            let _ = ssh_run(&server.user, &server.host, &cmd);
        }
        AccessType::Api => {
            if let Some((ticket, csrf)) = api_get_ticket_and_csrf(server) {
                if let Some(node) = api_get_node(server, &ticket) {
                    let res = if c.kind == "vm" { "qemu" } else { "lxc" };
                    api_post(server, &ticket, &csrf,
                        &format!("/nodes/{node}/{res}/{}/status/start", c.vmid));
                }
            }
        }
    }
}

pub fn stop_container(server: &ProxmoxServer, c: &Container) {
    match server.access {
        AccessType::Ssh => {
            let cmd = if c.kind == "vm" {
                format!("qm stop {}", c.vmid)
            } else {
                format!("pct stop {}", c.vmid)
            };
            let _ = ssh_run(&server.user, &server.host, &cmd);
        }
        AccessType::Api => {
            if let Some((ticket, csrf)) = api_get_ticket_and_csrf(server) {
                if let Some(node) = api_get_node(server, &ticket) {
                    let res = if c.kind == "vm" { "qemu" } else { "lxc" };
                    api_post(server, &ticket, &csrf,
                        &format!("/nodes/{node}/{res}/{}/status/stop", c.vmid));
                }
            }
        }
    }
}

/// Console command. API-only servers can't do console — returns None.
pub fn console_cmd(server: &ProxmoxServer, c: &Container) -> Option<String> {
    if server.access == AccessType::Api { return None; }
    let enter = if c.kind == "vm" {
        format!("qm terminal {}", c.vmid)
    } else {
        format!("pct enter {}", c.vmid)
    };
    Some(format!("ssh -t {}@{} {enter}", server.user, server.host))
}

// ── Display ──

pub fn display_server(s: &ProxmoxServer) -> Line<'static> {
    let tag = match s.access {
        AccessType::Ssh => "ssh",
        AccessType::Api => "api",
    };
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
        Span::styled(
            format!("  [{tag}]"),
            Style::default().fg(Color::Rgb(92, 99, 112)),
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
                    password: None,
                    port: None,                },
                SshEntry {
                    name: "web".into(),
                    host: "5.6.7.8".into(),
                    user: None,
                    emoji: "🌐".into(),
                    fg: String::new(),
                    bg: String::new(),
                    r#type: "ssh".into(),
                    password: None,
                    port: None,                },
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
            access: AccessType::Ssh,
            password: None,
            port: 8006,
        };
        let c = Container {
            vmid: 101,
            name: "test".into(),
            status: "running".into(),
            kind: "lxc".into(),
        };
        assert!(console_cmd(&s, &c).unwrap().contains("pct enter 101"));
    }

    #[test]
    fn console_cmd_vm() {
        let s = ProxmoxServer {
            name: "pve".into(),
            host: "1.2.3.4".into(),
            user: "root".into(),
            access: AccessType::Ssh,
            password: None,
            port: 8006,
        };
        let c = Container {
            vmid: 100,
            name: "win".into(),
            status: "running".into(),
            kind: "vm".into(),
        };
        assert!(console_cmd(&s, &c).unwrap().contains("qm terminal 100"));
    }
}
