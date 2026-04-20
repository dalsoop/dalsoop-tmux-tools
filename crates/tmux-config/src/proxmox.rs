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
    pub image: String,
    pub status: String,
    pub ports: String,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub name: String,
    pub date: String,
}

#[derive(Clone, Debug)]
pub struct Resources {
    pub cpus: String,
    pub mem_used: String,
    pub mem_total: String,
    pub _disk_used: String,
    pub disk_total: String,
    pub uptime: String,
    pub ip: String,
}

/// All detail info for a container, fetched at once.
#[derive(Clone, Debug)]
pub struct DetailInfo {
    pub resources: Option<Resources>,
    pub docker: Vec<DockerContainer>,
    pub ports: Vec<PortInfo>,
    pub snapshots: Vec<Snapshot>,
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

// ── SSH host key ──

/// Check if `host` has an entry in known_hosts.
pub fn host_key_exists(host: &str) -> bool {
    Command::new("ssh-keygen")
        .args(["-F", host])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Run `ssh-keyscan` and append the result to known_hosts.
/// Returns true on success.
pub fn register_host_key(host: &str) -> bool {
    let output = Command::new("ssh-keyscan")
        .args(["-H", host])
        .output();
    match output {
        Ok(o) if !o.stdout.is_empty() => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into()); // LINT_ALLOW: last-resort fallback when $HOME is unset
            let known_hosts = std::path::PathBuf::from(home).join(".ssh/known_hosts");
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&known_hosts)
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(&o.stdout)
                })
                .is_ok()
        }
        _ => false,
    }
}

/// Ask the user whether to register the host key, then do it.
/// Returns true if the key was registered (or already existed).
pub fn ensure_host_key(host: &str) -> bool {
    if host_key_exists(host) {
        return true;
    }
    eprint!("Host key not found for '{host}'. Register it now? [y/N] ");
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    if !input.trim().eq_ignore_ascii_case("y") {
        return false;
    }
    if register_host_key(host) {
        eprintln!("✓ Host key registered for '{host}'");
        true
    } else {
        eprintln!("✗ Failed to register host key for '{host}'");
        false
    }
}

// ── SSH commands ──

/// 호스트 문자열이 현재 머신(로컬)을 가리키는지 판정.
/// 루프백·localhost·현재 hostname 이면 true.
pub fn is_localhost(host: &str) -> bool {
    if matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return true;
    }
    if let Ok(out) = Command::new("hostname").output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if host == s.trim() {
                return true;
            }
        }
    }
    false
}

fn ssh_run(user: &str, host: &str, cmd: &str) -> Option<String> {
    // 호스트가 자기 자신이면 SSH 우회해 로컬 실행.
    if is_localhost(host) {
        let _ = user; // sudo/su 전환 없이 현재 사용자로 실행 (TUI 는 대개 root)
        let output = Command::new("sh").arg("-c").arg(cmd).output().ok()?;
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
        return None;
    }
    let target = format!("{user}@{host}");
    let output = Command::new("ssh")
        .args(SSH_OPTS)
        .arg(&target)
        .arg(cmd)
        .output()
        .ok()?;
    if output.status.success() {
        return Some(String::from_utf8_lossy(&output.stdout).to_string());
    }
    // Detect host key verification failure and offer to fix it
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Host key verification failed") && ensure_host_key(host) {
        // Retry after registering the key
        let output = Command::new("ssh")
            .args(SSH_OPTS)
            .arg(&target)
            .arg(cmd)
            .output()
            .ok()?;
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }
    None
}

/// ssh 래퍼가 필요한 커맨드를 로컬·원격 상황에 맞게 조립.
fn remote_or_local(user: &str, host: &str, remote_cmd: &str) -> String {
    if is_localhost(host) {
        remote_cmd.to_string()
    } else {
        format!("ssh -t {user}@{host} {remote_cmd}")
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
    if server.access == AccessType::Api { return Vec::new(); }
    let cmd = format!(
        "pct exec {vmid} -- docker ps --format '{{{{.Names}}}}\\t{{{{.Image}}}}\\t{{{{.Status}}}}\\t{{{{.Ports}}}}' 2>/dev/null"
    );
    let out = match ssh_run(&server.user, &server.host, &cmd) {
        Some(o) => o,
        None => return Vec::new(),
    };

    out.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let parts: Vec<&str> = l.splitn(4, '\t').collect();
            DockerContainer {
                name: parts.first().unwrap_or(&"").to_string(),
                image: parts.get(1).unwrap_or(&"").to_string(),
                status: parts.get(2).unwrap_or(&"").to_string(),
                ports: parts.get(3).unwrap_or(&"").to_string(),
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct PortInfo {
    pub addr: String,
    pub port: u16,
    pub process: String,
    pub proto_guess: String, // http, https, ssh, ws, db, etc.
}

fn guess_protocol(port: u16, process: &str) -> String {
    let p = process.to_lowercase();
    // Process name hints
    if p.contains("nginx") || p.contains("apache") || p.contains("caddy") || p.contains("traefik") {
        if port == 443 || port == 8443 { return "https".into(); }
        return "http".into();
    }
    if p.contains("sshd") { return "ssh".into(); }
    if p.contains("postgres") { return "postgresql".into(); }
    if p.contains("mysql") || p.contains("mariadbd") { return "mysql".into(); }
    if p.contains("redis") { return "redis".into(); }
    if p.contains("mongo") { return "mongodb".into(); }
    // Port number hints
    match port {
        22 => "ssh".into(),
        25 | 465 | 587 => "smtp".into(),
        53 => "dns".into(),
        80 | 8080 | 8000 | 3000 | 5000 => "http".into(),
        443 | 8443 => "https".into(),
        993 => "imaps".into(),
        143 => "imap".into(),
        3306 => "mysql".into(),
        5432 => "postgresql".into(),
        6379 => "redis".into(),
        27017 => "mongodb".into(),
        8006 => "proxmox".into(),
        9090 | 9100 | 3100 | 9200 => "metrics".into(),
        _ => "tcp".into(),
    }
}

pub fn fetch_ports(server: &ProxmoxServer, vmid: u32) -> Vec<PortInfo> {
    if server.access == AccessType::Api { return Vec::new(); }
    let cmd = format!(
        "pct exec {vmid} -- ss -tlnp 2>/dev/null | tail -n+2"
    );
    let out = match ssh_run(&server.user, &server.host, &cmd) {
        Some(o) => o,
        None => return Vec::new(),
    };

    out.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            let local = cols.get(3)?;
            // Extract process name from users:(("name",...))
            let process = cols.get(5..)
                .map(|rest| rest.join(" "))
                .and_then(|s| {
                    let start = s.find("((\"")? + 3;
                    let end = s[start..].find('"')? + start;
                    Some(s[start..end].to_string())
                })
                .unwrap_or_default();
            // Parse addr:port
            let (addr, port_str) = if let Some(idx) = local.rfind(':') {
                (local[..idx].to_string(), &local[idx+1..])
            } else {
                (local.to_string(), "0")
            };
            let port: u16 = port_str.parse().unwrap_or(0);
            let proto_guess = guess_protocol(port, &process);
            Some(PortInfo { addr, port, process, proto_guess })
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
    Some(remote_or_local(&server.user, &server.host, &enter))
}

// ── Detail fetch (combined) ──

pub fn fetch_detail(server: &ProxmoxServer, c: &Container) -> DetailInfo {
    DetailInfo {
        resources: fetch_resources(server, c),
        docker: fetch_docker(server, c.vmid),
        ports: fetch_ports(server, c.vmid),
        snapshots: fetch_snapshots(server, c),
    }
}

fn fetch_resources(server: &ProxmoxServer, c: &Container) -> Option<Resources> {
    if server.access == AccessType::Api { return None; }
    let kind = if c.kind == "vm" { "qm" } else { "pct" };
    let cmd = format!(
        r#"{kind} status {vmid} 2>/dev/null && {kind} config {vmid} 2>/dev/null | grep -E '^(cores|memory|rootfs|net)'"#,
        vmid = c.vmid
    );
    let out = ssh_run(&server.user, &server.host, &cmd)?;

    let mut cpus = "?".to_string();
    let mut mem_total = "?".to_string();
    let mut disk_total = "?".to_string();
    let mut mem_used = "?".to_string();
    let mut uptime = "?".to_string();
    let mut ip = "-".to_string();

    for line in out.lines() {
        if line.starts_with("cores:") {
            cpus = line.split(':').nth(1).unwrap_or("?").trim().to_string();
        } else if line.starts_with("memory:") {
            mem_total = format!("{}MB", line.split(':').nth(1).unwrap_or("?").trim());
        } else if line.contains("rootfs:") {
            if let Some(size) = line.split("size=").nth(1) {
                disk_total = size.split(',').next().unwrap_or("?").trim().to_string();
            }
        } else if line.starts_with("status:") {
            // skip
        } else if line.contains("mem:") {
            let val = line.split(':').nth(1).unwrap_or("0").trim();
            if let Ok(bytes) = val.parse::<u64>() {
                mem_used = format!("{}MB", bytes / 1024 / 1024);
            }
        } else if line.contains("uptime:") {
            let val = line.split(':').nth(1).unwrap_or("0").trim();
            if let Ok(secs) = val.parse::<u64>() {
                let h = secs / 3600;
                let d = h / 24;
                if d > 0 { uptime = format!("{d}d {h}h", h = h % 24); }
                else { uptime = format!("{h}h"); }
            }
        }
    }

    // Get IP
    if c.kind == "lxc" {
        if let Some(ip_out) = ssh_run(&server.user, &server.host,
            &format!("pct exec {} -- hostname -I 2>/dev/null", c.vmid))
        {
            ip = ip_out.split_whitespace().next().unwrap_or("-").to_string();
        }
    }

    Some(Resources { cpus, mem_used, mem_total, _disk_used: "-".into(), disk_total, uptime, ip })
}

fn fetch_snapshots(server: &ProxmoxServer, c: &Container) -> Vec<Snapshot> {
    if server.access == AccessType::Api { return Vec::new(); }
    let kind = if c.kind == "vm" { "qm" } else { "pct" };
    let cmd = format!("{kind} listsnapshot {} 2>/dev/null", c.vmid);
    let out = match ssh_run(&server.user, &server.host, &cmd) {
        Some(o) => o,
        None => return Vec::new(),
    };
    out.lines()
        .filter(|l| !l.trim().is_empty() && !l.contains("current"))
        .map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            let name = parts.first().map(|s| s.trim_start_matches('`').trim_end_matches('`'))
                .unwrap_or("?").to_string();
            let date = parts.get(1).unwrap_or(&"").to_string();
            Snapshot { name, date }
        })
        .filter(|s| s.name != "current" && !s.name.is_empty())
        .collect()
}

// ── Host info ──

#[derive(Clone, Debug)]
pub struct HostInfo {
    pub hostname: String,
    pub kernel: String,
    pub cpu_model: String,
    pub cpu_cores: String,
    pub mem_used: String,
    pub mem_total: String,
    pub uptime: String,
    pub load: String,
    pub pve_version: String,
}

pub fn fetch_host_info(server: &ProxmoxServer) -> Option<HostInfo> {
    let cmd = r#"echo "hostname:$(hostname)" && echo "kernel:$(uname -r)" && echo "cpu:$(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | xargs)" && echo "cores:$(nproc)" && echo "mem:$(free -m | awk '/Mem/{print $3"/"$2"MB"}')" && echo "uptime:$(uptime -p 2>/dev/null || uptime)" && echo "load:$(cat /proc/loadavg | awk '{print $1,$2,$3}')" && echo "pve:$(pveversion 2>/dev/null || echo '-')"
"#;
    let out = ssh_run(&server.user, &server.host, cmd)?;
    let mut info = HostInfo {
        hostname: String::new(), kernel: String::new(), cpu_model: String::new(),
        cpu_cores: String::new(), mem_used: String::new(), mem_total: String::new(),
        uptime: String::new(), load: String::new(), pve_version: String::new(),
    };
    for line in out.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim().to_string();
            match k.trim() {
                "hostname" => info.hostname = v,
                "kernel" => info.kernel = v,
                "cpu" => info.cpu_model = v,
                "cores" => info.cpu_cores = v,
                "mem" => {
                    let parts: Vec<&str> = v.split('/').collect();
                    info.mem_used = parts.first().unwrap_or(&"?").to_string();
                    info.mem_total = parts.get(1).unwrap_or(&"?").to_string();
                }
                "uptime" => info.uptime = v,
                "load" => info.load = v,
                "pve" => info.pve_version = v,
                _ => {}
            }
        }
    }
    Some(info)
}

pub fn display_host_info(info: &HostInfo) -> Vec<Line<'static>> {
    let header = Style::default().fg(Color::Rgb(97, 175, 239)).add_modifier(ratatui::style::Modifier::BOLD);
    let dim = Style::default().fg(Color::Rgb(92, 99, 112));
    let val = Style::default().fg(Color::Rgb(171, 178, 191));

    vec![
        Line::from(Span::styled("  🖥️ Host", header)),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled(format!("{} ", info.hostname), Style::default().fg(Color::Rgb(152, 195, 121))),
            Span::styled(format!("| {} | {} cores ", info.cpu_model, info.cpu_cores), val),
        ]),
        Line::from(vec![
            Span::styled("     RAM: ", dim),
            Span::styled(format!("{}/{}", info.mem_used, info.mem_total), val),
            Span::styled("  |  Load: ", dim),
            Span::styled(info.load.clone(), val),
            Span::styled("  |  Kernel: ", dim),
            Span::styled(info.kernel.clone(), val),
        ]),
        Line::from(vec![
            Span::styled("     Uptime: ", dim),
            Span::styled(info.uptime.clone(), val),
            Span::styled("  |  PVE: ", dim),
            Span::styled(info.pve_version.clone(), val),
        ]),
        Line::from(""),
    ]
}

// ── LXC/VM Management ──

/// Get next available VMID on a server.
pub fn next_vmid(server: &ProxmoxServer) -> Option<u32> {
    match server.access {
        AccessType::Ssh => {
            let out = ssh_run(&server.user, &server.host, "pvesh get /cluster/nextid 2>/dev/null")?;
            out.trim().trim_matches('"').parse().ok()
        }
        AccessType::Api => {
            let ticket = api_get_ticket(server)?;
            let text = api_get(server, &ticket, "/cluster/nextid")?;
            let start = text.find("\"data\":")?;
            let rest = &text[start + 7..];
            // Could be number or "number"
            let val = rest.trim().trim_matches(|c| c == '"' || c == '}' || c == ' ');
            val.parse().ok()
        }
    }
}

/// List available LXC templates on a server.
pub fn list_templates(server: &ProxmoxServer) -> Vec<String> {
    let cmd = "pveam available --section system 2>/dev/null | awk '{print $2}' | head -20";
    ssh_run(&server.user, &server.host, cmd)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// List downloaded templates on a server's local storage.
pub fn list_local_templates(server: &ProxmoxServer) -> Vec<String> {
    let cmd = "pveam list local 2>/dev/null | tail -n+2 | awk '{print $1}'";
    ssh_run(&server.user, &server.host, cmd)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// Create an LXC container.
#[allow(clippy::too_many_arguments)]
pub fn create_lxc(server: &ProxmoxServer, vmid: u32, hostname: &str, template: &str,
                  memory: u32, cores: u32, disk: u32, password: &str) -> bool {
    let cmd = format!(
        "pct create {vmid} {template} \
         --hostname {hostname} \
         --memory {memory} --cores {cores} \
         --rootfs local-lvm:{disk} \
         --net0 name=eth0,bridge=vmbr0,ip=dhcp \
         --password {password} \
         --start 1 \
         --unprivileged 1 2>&1"
    );
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

/// Clone an existing container/VM.
pub fn clone_ct(server: &ProxmoxServer, src_vmid: u32, new_vmid: u32, hostname: &str, kind: &str) -> bool {
    let cmd = if kind == "vm" {
        format!("qm clone {src_vmid} {new_vmid} --name {hostname} --full 2>&1")
    } else {
        format!("pct clone {src_vmid} {new_vmid} --hostname {hostname} --full 2>&1")
    };
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

/// Delete a container/VM (must be stopped).
pub fn delete_ct(server: &ProxmoxServer, c: &Container) -> bool {
    let cmd = if c.kind == "vm" {
        format!("qm destroy {} --purge 2>&1", c.vmid)
    } else {
        format!("pct destroy {} --purge 2>&1", c.vmid)
    };
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

// ── Docker management ──

pub fn docker_start(server: &ProxmoxServer, vmid: u32, container: &str) -> bool {
    let cmd = format!("pct exec {vmid} -- docker start {container} 2>&1");
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

pub fn docker_stop(server: &ProxmoxServer, vmid: u32, container: &str) -> bool {
    let cmd = format!("pct exec {vmid} -- docker stop {container} 2>&1");
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

pub fn docker_restart(server: &ProxmoxServer, vmid: u32, container: &str) -> bool {
    let cmd = format!("pct exec {vmid} -- docker restart {container} 2>&1");
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

/// Returns a tmux command to tail docker logs.
pub fn docker_logs_cmd(server: &ProxmoxServer, vmid: u32, container: &str) -> String {
    let inner = format!("pct exec {vmid} -- docker logs -f --tail 100 {container}");
    remote_or_local(&server.user, &server.host, &inner)
}

/// Returns a tmux command to tail container system logs.
pub fn container_logs_cmd(server: &ProxmoxServer, c: &Container) -> Option<String> {
    if server.access == AccessType::Api { return None; }
    let inner = if c.kind == "vm" {
        // VM: serial console log
        format!("qm terminal {}", c.vmid)
    } else {
        // LXC: journalctl or syslog
        format!(
            "pct exec {} -- sh -c 'journalctl -f -n 100 2>/dev/null || tail -f /var/log/syslog 2>/dev/null || tail -f /var/log/messages'",
            c.vmid
        )
    };
    Some(remote_or_local(&server.user, &server.host, &inner))
}

/// Returns a tmux command to exec into docker container.
pub fn docker_exec_cmd(server: &ProxmoxServer, vmid: u32, container: &str) -> String {
    let inner = format!(
        "pct exec {vmid} -- docker exec -it {container} sh -c 'bash 2>/dev/null || sh'"
    );
    remote_or_local(&server.user, &server.host, &inner)
}

// ── Display ──

pub fn display_server(s: &ProxmoxServer) -> Line<'static> {
    let tag = match s.access {
        AccessType::Ssh if is_localhost(&s.host) => "local",
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

/// Build all display lines for the detail view (depth 2).
pub fn display_detail(info: &DetailInfo) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let header = Style::default().fg(Color::Rgb(97, 175, 239)).add_modifier(ratatui::style::Modifier::BOLD);
    let dim = Style::default().fg(Color::Rgb(92, 99, 112));
    let val = Style::default().fg(Color::Rgb(171, 178, 191));
    let green = Style::default().fg(Color::Rgb(152, 195, 121));
    let red = Style::default().fg(Color::Rgb(224, 108, 117));
    let yellow = Style::default().fg(Color::Rgb(229, 192, 123));

    // Resources
    lines.push(Line::from(Span::styled("  📊 Resources", header)));
    if let Some(r) = &info.resources {
        lines.push(Line::from(vec![
            Span::styled("     CPU: ", dim),
            Span::styled(format!("{} cores", r.cpus), val),
            Span::styled("  |  RAM: ", dim),
            Span::styled(format!("{}/{}", r.mem_used, r.mem_total), val),
            Span::styled("  |  Disk: ", dim),
            Span::styled(r.disk_total.clone(), val),
        ]));
        lines.push(Line::from(vec![
            Span::styled("     Uptime: ", dim),
            Span::styled(r.uptime.clone(), val),
            Span::styled("  |  IP: ", dim),
            Span::styled(r.ip.clone(), val),
        ]));
    } else {
        lines.push(Line::from(Span::styled("     (not available)", dim)));
    }
    lines.push(Line::from(""));

    // Docker — each container with its individual ports
    lines.push(Line::from(Span::styled(
        format!("  🐳 Docker ({})", info.docker.len()), header
    )));
    if info.docker.is_empty() {
        lines.push(Line::from(Span::styled("     (none or no docker)", dim)));
    } else {
        for d in &info.docker {
            let color = if d.status.starts_with("Up") { green } else { red };
            // Container name + image + status
            lines.push(Line::from(vec![
                Span::styled(format!("     {:<20}", d.name), color),
                Span::styled(format!(" {:<30}", d.image), dim),
                Span::styled(d.status.clone(), val),
            ]));
            // Individual port mappings
            if !d.ports.is_empty() {
                for port_entry in d.ports.split(", ") {
                    let proto = if port_entry.contains("443") { "https" }
                        else if port_entry.contains(":80") || port_entry.contains("->80")
                             || port_entry.contains(":8080") || port_entry.contains("->8080")
                             || port_entry.contains(":3000") || port_entry.contains("->3000") { "http" }
                        else if port_entry.contains(":6379") { "redis" }
                        else if port_entry.contains(":5432") { "postgresql" }
                        else if port_entry.contains(":3306") { "mysql" }
                        else if port_entry.contains(":27017") { "mongodb" }
                        else { "tcp" };
                    lines.push(Line::from(vec![
                        Span::styled("       ↳ ", dim),
                        Span::styled(port_entry.trim().to_string(), yellow),
                        Span::styled(format!("  ({proto})"), dim),
                    ]));
                }
            }
        }
    }
    lines.push(Line::from(""));

    // Listening Ports — with process + protocol
    lines.push(Line::from(Span::styled(
        format!("  🔌 Listening Ports ({})", info.ports.len()), header
    )));
    if info.ports.is_empty() {
        lines.push(Line::from(Span::styled("     (none)", dim)));
    } else {
        for p in &info.ports {
            let proto_color = match p.proto_guess.as_str() {
                "https" => green,
                "http" => Style::default().fg(Color::Rgb(97, 175, 239)),
                "ssh" => Style::default().fg(Color::Rgb(198, 120, 221)),
                "mysql" | "postgresql" | "redis" | "mongodb" => yellow,
                _ => val,
            };
            lines.push(Line::from(vec![
                Span::styled(format!("     {:<25}", format!("{}:{}", p.addr, p.port)), val),
                Span::styled(format!("{:<12}", p.proto_guess), proto_color),
                Span::styled(p.process.clone(), dim),
            ]));
        }
    }
    lines.push(Line::from(""));

    // Snapshots
    lines.push(Line::from(Span::styled(
        format!("  📸 Snapshots ({})", info.snapshots.len()), header
    )));
    if info.snapshots.is_empty() {
        lines.push(Line::from(Span::styled("     (none)", dim)));
    } else {
        for s in &info.snapshots {
            lines.push(Line::from(vec![
                Span::styled(format!("     {:<20}", s.name), val),
                Span::styled(s.date.clone(), dim),
            ]));
        }
    }

    lines
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
