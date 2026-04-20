//! Windowbar 설정에서 ProxmoxServer 목록을 뽑고, 로컬 노드가 Proxmox 클러스터의
//! 일부면 `/etc/pve/corosync.conf` 를 읽어 peer 노드를 자동으로 붙인다.

use tmux_windowbar::config::template::Config;

use super::parse::parse_cluster_members;
use super::ssh::is_localhost;
use super::types::{AccessType, ProxmoxServer};

pub fn get_servers(config: &Config) -> Vec<ProxmoxServer> {
    let mut servers: Vec<ProxmoxServer> = config
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
        .collect();

    let local_idx = servers
        .iter()
        .position(|s| s.access == AccessType::Ssh && is_localhost(&s.host));
    if let Some(idx) = local_idx {
        if let Some(members) = cluster_members() {
            let default_user = servers[idx].user.clone();
            let default_port = servers[idx].port;
            for (name, addr) in members {
                if is_localhost(&addr) { continue; }
                if servers.iter().any(|s| s.name == name || s.host == addr) { continue; }
                servers.push(ProxmoxServer {
                    name,
                    host: addr,
                    user: default_user.clone(),
                    access: AccessType::Ssh,
                    password: None,
                    port: default_port,
                });
            }
        }
    }

    servers
}

fn cluster_members() -> Option<Vec<(String, String)>> {
    let text = std::fs::read_to_string("/etc/pve/corosync.conf").ok()?;
    let m = parse_cluster_members(&text);
    if m.is_empty() { None } else { Some(m) }
}
