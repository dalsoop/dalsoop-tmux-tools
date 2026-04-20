//! Proxmox 도메인 - CLI / TUI 양쪽에서 쓰는 공개 API.
//!
//! 내부는 `types / parse / ssh / api / cache / cluster / client / display` 로
//! 세분화돼 있고, 호출자는 대체로 이 mod 의 wrapper 만 본다.

pub mod api;
pub mod cache;
pub mod client;
pub mod cluster;
pub mod display;
pub mod parse;
pub mod ssh;
pub mod types;

// ── Public re-exports ──

pub use client::ProxmoxClient;
pub use cluster::get_servers;
pub use display::{display_container, display_detail, display_host_info, display_server};
pub use ssh::{ensure_host_key, host_key_exists, is_localhost};
pub use types::{AccessType, Container, DetailInfo, HostInfo, ProxmoxServer};
// 일부 타입(DockerContainer/PortInfo/Resources/Snapshot/register_host_key) 은
// 현재 외부에서 직접 참조되지 않지만, 공개 API 일관성을 위해 submodule 경로
// (`types::DockerContainer` 등) 로 접근 가능한 상태를 유지한다.

// 내부 모듈에서만 쓰는 heavy-weight 기능을 로컬 use 로 가져온다.
use api::{api_get, api_get_ticket};
use cache::{cache_key, container_cache, host_info_cache, invalidate_containers};
use client::client_for;
use ssh::ssh_run;

// ── Fetch (cached wrappers) ──

pub fn fetch_containers(server: &ProxmoxServer) -> Vec<Container> {
    let key = cache_key(server);
    if let Some(hit) = container_cache().get(&key) {
        return hit;
    }

    let mut result = client_for(server).fetch_containers_raw();

    // Sort: running first, then by vmid
    result.sort_by(|a, b| {
        let ar = a.status == "running";
        let br = b.status == "running";
        br.cmp(&ar).then(a.vmid.cmp(&b.vmid))
    });

    container_cache().put(key, result.clone());
    result
}

pub fn fetch_host_info(server: &ProxmoxServer) -> Option<HostInfo> {
    let key = cache_key(server);
    if let Some(hit) = host_info_cache().get(&key) {
        return Some(hit);
    }
    let info = client_for(server).host_info()?;
    host_info_cache().put(key, info.clone());
    Some(info)
}

pub fn fetch_detail(server: &ProxmoxServer, c: &Container) -> DetailInfo {
    let client = client_for(server);
    DetailInfo {
        resources: client.resources(c),
        docker: client.docker(c.vmid),
        ports: client.ports(c.vmid),
        snapshots: client.snapshots(c),
    }
}

// ── Actions ──

pub fn start_container(server: &ProxmoxServer, c: &Container) {
    client_for(server).start(c);
    invalidate_containers(server);
}

pub fn stop_container(server: &ProxmoxServer, c: &Container) {
    client_for(server).stop(c);
    invalidate_containers(server);
}

/// 콘솔 명령. API-only 서버는 `None`.
pub fn console_cmd(server: &ProxmoxServer, c: &Container) -> Option<String> {
    client_for(server).console(c)
}

// ── Docker / 로그 명령 ──

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

/// docker logs 를 따라가는 tmux 명령. API-only 서버면 빈 문자열.
pub fn docker_logs_cmd(server: &ProxmoxServer, vmid: u32, container: &str) -> String {
    client_for(server).docker_logs_cmd(vmid, container).unwrap_or_default()
}

/// 컨테이너 시스템 로그를 따라가는 tmux 명령.
pub fn container_logs_cmd(server: &ProxmoxServer, c: &Container) -> Option<String> {
    client_for(server).container_logs_cmd(c)
}

/// docker exec -it 로 들어가는 tmux 명령. API-only 면 빈 문자열.
pub fn docker_exec_cmd(server: &ProxmoxServer, vmid: u32, container: &str) -> String {
    client_for(server).docker_exec_cmd(vmid, container).unwrap_or_default()
}

// ── LXC/VM 관리 (ssh 전용 경로) ──

/// 서버의 다음 가용 VMID 를 얻음.
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
            let val = rest.trim().trim_matches(|c| c == '"' || c == '}' || c == ' ');
            val.parse().ok()
        }
    }
}

pub fn list_templates(server: &ProxmoxServer) -> Vec<String> {
    let cmd = "pveam available --section system 2>/dev/null | awk '{print $2}' | head -20";
    ssh_run(&server.user, &server.host, cmd)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

pub fn list_local_templates(server: &ProxmoxServer) -> Vec<String> {
    let cmd = "pveam list local 2>/dev/null | tail -n+2 | awk '{print $1}'";
    ssh_run(&server.user, &server.host, cmd)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

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

pub fn clone_ct(server: &ProxmoxServer, src_vmid: u32, new_vmid: u32, hostname: &str, kind: &str) -> bool {
    let cmd = if kind == "vm" {
        format!("qm clone {src_vmid} {new_vmid} --name {hostname} --full 2>&1")
    } else {
        format!("pct clone {src_vmid} {new_vmid} --hostname {hostname} --full 2>&1")
    };
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

pub fn delete_ct(server: &ProxmoxServer, c: &Container) -> bool {
    let cmd = if c.kind == "vm" {
        format!("qm destroy {} --purge 2>&1", c.vmid)
    } else {
        format!("pct destroy {} --purge 2>&1", c.vmid)
    };
    ssh_run(&server.user, &server.host, &cmd).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmux_windowbar::config::template::SshEntry;

    #[test]
    fn get_servers_filters_proxmox_type() {
        let config = tmux_windowbar::config::template::Config {
            ssh: vec![
                SshEntry {
                    name: "pve".into(), host: "1.2.3.4".into(),
                    user: Some("root".into()),
                    emoji: "🖥️".into(), fg: String::new(), bg: String::new(),
                    r#type: "proxmox".into(), password: None, port: None,
                },
                SshEntry {
                    name: "web".into(), host: "5.6.7.8".into(),
                    user: None,
                    emoji: "🌐".into(), fg: String::new(), bg: String::new(),
                    r#type: "ssh".into(), password: None, port: None,
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
            vmid: 101, name: "gitlab".into(),
            status: "running".into(), kind: "lxc".into(),
        };
        let line = display_container(&c);
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(text.contains("101"));
        assert!(text.contains("gitlab"));
    }
}
