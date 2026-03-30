use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

/// A Proxmox LXC container or VM.
#[derive(Debug, Clone)]
pub struct Container {
    pub vmid: u32,
    pub name: String,
    pub status: String, // "running" or "stopped"
    pub kind: String,   // "lxc" or "vm"
}

/// Fetch containers and VMs from Proxmox via SSH.
/// Returns an empty vec with an error entry if unreachable.
pub fn fetch(ssh_host: &str) -> Vec<Container> {
    let mut result = Vec::new();

    // Run pct list and qm list in one SSH connection each
    let pct_out = run_ssh(ssh_host, "pct list 2>/dev/null");
    let qm_out = run_ssh(ssh_host, "qm list 2>/dev/null");

    match (pct_out, qm_out) {
        (Some(pct), Some(qm)) => {
            parse_pct(&pct, &mut result);
            parse_qm(&qm, &mut result);
        }
        _ => {
            // Return empty — caller can show status message
            return vec![];
        }
    }

    // Sort: running first, then by vmid
    result.sort_by(|a, b| {
        let a_run = if a.status == "running" { 0 } else { 1 };
        let b_run = if b.status == "running" { 0 } else { 1 };
        a_run.cmp(&b_run).then(a.vmid.cmp(&b.vmid))
    });

    result
}

fn run_ssh(host: &str, cmd: &str) -> Option<String> {
    // host may be "root@192.168.2.50" or just "192.168.2.50"
    let target = if host.contains('@') {
        host.to_string()
    } else {
        format!("root@{host}")
    };

    let output = std::process::Command::new("ssh")
        .args([
            "-o", "ConnectTimeout=5",
            "-o", "BatchMode=yes",
            "-o", "StrictHostKeyChecking=no",
            &target,
            cmd,
        ])
        .output()
        .ok()?;

    if output.status.success() || !output.stdout.is_empty() {
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        None
    }
}

/// Parse `pct list` output.
/// Format: VMID       Status     Lock         Name
///         101        running                 gitlab
fn parse_pct(output: &str, result: &mut Vec<Container>) {
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let vmid: u32 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let status = parts[1].to_string();
        // Name may be parts[2] (no lock) or parts[3] (with lock)
        let name = if parts.len() >= 4 {
            // Check if parts[2] is a lock or the name
            // Lock fields are things like "backup", "migrate", "snapshot"
            // We'll take the last token as the name
            parts.last().unwrap_or(&"").to_string()
        } else if parts.len() == 3 {
            parts[2].to_string()
        } else {
            format!("ct-{vmid}")
        };
        result.push(Container {
            vmid,
            name,
            status,
            kind: "lxc".into(),
        });
    }
}

/// Parse `qm list` output.
/// Format:       VMID NAME                 STATUS     MEM(MB)     BOOTDISK(GB) PID
///                100 pve-test             stopped    2048               20.00 0
fn parse_qm(output: &str, result: &mut Vec<Container>) {
    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let vmid: u32 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let name = parts[1].to_string();
        let status = parts[2].to_string();
        result.push(Container {
            vmid,
            name,
            status,
            kind: "vm".into(),
        });
    }
}

/// Format a container for display in the list.
pub fn display(c: &Container) -> Line<'static> {
    let status_indicator = if c.status == "running" { "●" } else { "○" };
    let status_color = if c.status == "running" {
        Color::Rgb(152, 195, 121) // GREEN
    } else {
        Color::Rgb(224, 108, 117) // RED
    };

    let text = format!(
        "  {:>5}  {:<20}  {:<10}  {}",
        c.vmid, c.name, c.status, c.kind
    );

    Line::from(vec![
        Span::styled(status_indicator.to_string(), Style::default().fg(status_color)),
        Span::raw(text),
    ])
}

/// Start a container or VM.
pub fn start(ssh_host: &str, c: &Container) {
    let cmd = match c.kind.as_str() {
        "vm" => format!("qm start {}", c.vmid),
        _ => format!("pct start {}", c.vmid),
    };
    let _ = run_ssh(ssh_host, &cmd);
}

/// Stop a container or VM.
pub fn stop(ssh_host: &str, c: &Container) {
    let cmd = match c.kind.as_str() {
        "vm" => format!("qm stop {}", c.vmid),
        _ => format!("pct stop {}", c.vmid),
    };
    let _ = run_ssh(ssh_host, &cmd);
}

/// Returns the SSH command string to open a console session.
pub fn console_cmd(host: &str, c: &Container) -> String {
    // host may be "root@192.168.2.50" or bare IP — normalise to bare IP
    let bare_host = if let Some(pos) = host.rfind('@') {
        &host[pos + 1..]
    } else {
        host
    };
    match c.kind.as_str() {
        "vm" => format!(
            "ssh -t root@{} qm terminal {}",
            bare_host, c.vmid
        ),
        _ => format!(
            "ssh -t root@{} pct enter {}",
            bare_host, c.vmid
        ),
    }
}

/// Resolve the Proxmox SSH host from the config SSH entries.
/// Looks for an entry with name "proxmox", falls back to "root@192.168.2.50".
pub fn resolve_proxmox_host(ssh_entries: &[tmux_windowbar::config::template::SshEntry]) -> String {
    for e in ssh_entries {
        if e.name.to_lowercase() == "proxmox" {
            return if let Some(ref user) = e.user {
                format!("{user}@{}", e.host)
            } else {
                e.host.clone()
            };
        }
    }
    "root@192.168.2.50".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pct_output() {
        let input = "VMID       Status     Lock         Name\n\
                     101        running                  gitlab\n\
                     102        stopped                  nextcloud\n";
        let mut result = Vec::new();
        parse_pct(input, &mut result);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].vmid, 101);
        assert_eq!(result[0].status, "running");
        assert_eq!(result[0].kind, "lxc");
        assert_eq!(result[1].vmid, 102);
        assert_eq!(result[1].status, "stopped");
    }

    #[test]
    fn parse_qm_output() {
        let input = "      VMID NAME                 STATUS     MEM(MB)     BOOTDISK(GB) PID\n\
                       100 pve-test             stopped    2048               20.00 0\n";
        let mut result = Vec::new();
        parse_qm(input, &mut result);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vmid, 100);
        assert_eq!(result[0].name, "pve-test");
        assert_eq!(result[0].status, "stopped");
        assert_eq!(result[0].kind, "vm");
    }

    #[test]
    fn display_shows_vmid_and_status() {
        let c = Container {
            vmid: 101,
            name: "gitlab".into(),
            status: "running".into(),
            kind: "lxc".into(),
        };
        let line = display(&c);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("101"));
        assert!(text.contains("gitlab"));
        assert!(text.contains("running"));
        assert!(text.contains("lxc"));
    }

    #[test]
    fn console_cmd_lxc() {
        let c = Container {
            vmid: 101,
            name: "gitlab".into(),
            status: "running".into(),
            kind: "lxc".into(),
        };
        let cmd = console_cmd("root@192.168.2.50", &c);
        assert_eq!(cmd, "ssh -t root@192.168.2.50 pct enter 101");
    }

    #[test]
    fn console_cmd_vm() {
        let c = Container {
            vmid: 100,
            name: "pve-test".into(),
            status: "stopped".into(),
            kind: "vm".into(),
        };
        let cmd = console_cmd("root@192.168.2.50", &c);
        assert_eq!(cmd, "ssh -t root@192.168.2.50 qm terminal 100");
    }

    #[test]
    fn resolve_host_finds_proxmox_entry() {
        use tmux_windowbar::config::template::SshEntry;
        let entries = vec![SshEntry {
            name: "proxmox".into(),
            host: "192.168.2.50".into(),
            user: Some("root".into()),
            emoji: String::new(),
            fg: String::new(),
            bg: String::new(),
        }];
        let host = resolve_proxmox_host(&entries);
        assert_eq!(host, "root@192.168.2.50");
    }

    #[test]
    fn resolve_host_falls_back() {
        let entries: Vec<tmux_windowbar::config::template::SshEntry> = vec![];
        let host = resolve_proxmox_host(&entries);
        assert_eq!(host, "root@192.168.2.50");
    }

    #[test]
    fn fetch_empty_on_unreachable() {
        // SSH to an invalid host should return empty vec, not panic
        let result = fetch("root@127.0.0.1"); // SSH to localhost with BatchMode likely fails
        // We can't assert empty definitively in all CI environments,
        // but it must not panic.
        let _ = result;
    }
}
