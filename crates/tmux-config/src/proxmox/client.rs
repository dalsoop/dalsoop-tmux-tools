//! `ProxmoxClient` trait + SSH/API 구현체 + `client_for` 팩토리.
//!
//! 호출부는 대개 `super`(mod.rs) 의 얇은 free fn wrapper 를 쓰고, 내부는
//! 전부 이 trait 위에서 돈다. `match AccessType` 분기는 `client_for` 한 곳에만.

use super::api::{
    api_fetch_containers, api_get_node, api_get_ticket_and_csrf, api_post,
};
use super::parse::{
    parse_docker, parse_host_info, parse_pct_list, parse_ports, parse_qm_list,
    parse_resources, parse_snapshots,
};
use super::ssh::{remote_or_local, ssh_run};
use super::types::{
    AccessType, Container, DockerContainer, HostInfo, PortInfo, ProxmoxServer,
    Resources, Snapshot,
};

pub trait ProxmoxClient {
    fn fetch_containers_raw(&self) -> Vec<Container>;
    fn start(&self, c: &Container);
    fn stop(&self, c: &Container);
    /// 로컬에서 대화형으로 띄울 명령. API-only 같이 불가능하면 `None`.
    fn console(&self, c: &Container) -> Option<String>;

    // ── 이하 default impl: API-only 서버는 전부 "지원 안 함" 을 반환 ──

    fn host_info(&self) -> Option<HostInfo> { None }
    fn resources(&self, _c: &Container) -> Option<Resources> { None }
    fn docker(&self, _vmid: u32) -> Vec<DockerContainer> { Vec::new() }
    fn ports(&self, _vmid: u32) -> Vec<PortInfo> { Vec::new() }
    fn snapshots(&self, _c: &Container) -> Vec<Snapshot> { Vec::new() }
    fn docker_logs_cmd(&self, _vmid: u32, _container: &str) -> Option<String> { None }
    fn container_logs_cmd(&self, _c: &Container) -> Option<String> { None }
    fn docker_exec_cmd(&self, _vmid: u32, _container: &str) -> Option<String> { None }
}

struct SshProxmoxClient<'a> {
    server: &'a ProxmoxServer,
}
struct ApiProxmoxClient<'a> {
    server: &'a ProxmoxServer,
}

impl SshProxmoxClient<'_> {
    fn run(&self, cmd: &str) -> Option<String> {
        ssh_run(&self.server.user, &self.server.host, cmd)
    }
}

impl ProxmoxClient for SshProxmoxClient<'_> {
    fn fetch_containers_raw(&self) -> Vec<Container> {
        let mut result = Vec::new();
        if let Some(out) = self.run("pct list 2>/dev/null") {
            result.extend(parse_pct_list(&out));
        }
        if let Some(out) = self.run("qm list 2>/dev/null") {
            result.extend(parse_qm_list(&out));
        }
        result
    }
    fn start(&self, c: &Container) {
        let cmd = if c.kind == "vm" { format!("qm start {}", c.vmid) } else { format!("pct start {}", c.vmid) };
        let _ = self.run(&cmd);
    }
    fn stop(&self, c: &Container) {
        let cmd = if c.kind == "vm" { format!("qm stop {}", c.vmid) } else { format!("pct stop {}", c.vmid) };
        let _ = self.run(&cmd);
    }
    fn console(&self, c: &Container) -> Option<String> {
        let enter = if c.kind == "vm" { format!("qm terminal {}", c.vmid) } else { format!("pct enter {}", c.vmid) };
        Some(remote_or_local(&self.server.user, &self.server.host, &enter))
    }

    fn host_info(&self) -> Option<HostInfo> {
        let cmd = r#"echo "hostname:$(hostname)" && echo "kernel:$(uname -r)" && echo "cpu:$(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | xargs)" && echo "cores:$(nproc)" && echo "mem:$(free -m | awk '/Mem/{print $3"/"$2"MB"}')" && echo "uptime:$(uptime -p 2>/dev/null || uptime)" && echo "load:$(cat /proc/loadavg | awk '{print $1,$2,$3}')" && echo "pve:$(pveversion 2>/dev/null || echo '-')"
"#;
        let out = self.run(cmd)?;
        Some(parse_host_info(&out))
    }

    fn resources(&self, c: &Container) -> Option<Resources> {
        let kind = if c.kind == "vm" { "qm" } else { "pct" };
        let cmd = format!(
            r#"{kind} status {vmid} 2>/dev/null && {kind} config {vmid} 2>/dev/null | grep -E '^(cores|memory|rootfs|net)'"#,
            vmid = c.vmid
        );
        let out = self.run(&cmd)?;
        let mut res = parse_resources(&out);
        if c.kind == "lxc" {
            if let Some(ip_out) = self.run(&format!("pct exec {} -- hostname -I 2>/dev/null", c.vmid)) {
                res.ip = ip_out.split_whitespace().next().unwrap_or("-").to_string();
            }
        }
        Some(res)
    }

    fn docker(&self, vmid: u32) -> Vec<DockerContainer> {
        let cmd = format!(
            "pct exec {vmid} -- docker ps --format '{{{{.Names}}}}\\t{{{{.Image}}}}\\t{{{{.Status}}}}\\t{{{{.Ports}}}}' 2>/dev/null"
        );
        self.run(&cmd).map(|o| parse_docker(&o)).unwrap_or_default()
    }

    fn ports(&self, vmid: u32) -> Vec<PortInfo> {
        let cmd = format!("pct exec {vmid} -- ss -tlnp 2>/dev/null | tail -n+2");
        self.run(&cmd).map(|o| parse_ports(&o)).unwrap_or_default()
    }

    fn snapshots(&self, c: &Container) -> Vec<Snapshot> {
        let kind = if c.kind == "vm" { "qm" } else { "pct" };
        let cmd = format!("{kind} listsnapshot {} 2>/dev/null", c.vmid);
        self.run(&cmd).map(|o| parse_snapshots(&o)).unwrap_or_default()
    }

    fn docker_logs_cmd(&self, vmid: u32, container: &str) -> Option<String> {
        let inner = format!("pct exec {vmid} -- docker logs -f --tail 100 {container}");
        Some(remote_or_local(&self.server.user, &self.server.host, &inner))
    }

    fn container_logs_cmd(&self, c: &Container) -> Option<String> {
        let inner = if c.kind == "vm" {
            format!("qm terminal {}", c.vmid)
        } else {
            format!(
                "pct exec {} -- sh -c 'journalctl -f -n 100 2>/dev/null || tail -f /var/log/syslog 2>/dev/null || tail -f /var/log/messages'",
                c.vmid
            )
        };
        Some(remote_or_local(&self.server.user, &self.server.host, &inner))
    }

    fn docker_exec_cmd(&self, vmid: u32, container: &str) -> Option<String> {
        let inner = format!(
            "pct exec {vmid} -- docker exec -it {container} sh -c 'bash 2>/dev/null || sh'"
        );
        Some(remote_or_local(&self.server.user, &self.server.host, &inner))
    }
}

impl ProxmoxClient for ApiProxmoxClient<'_> {
    fn fetch_containers_raw(&self) -> Vec<Container> {
        api_fetch_containers(self.server)
    }
    fn start(&self, c: &Container) {
        if let Some((ticket, csrf)) = api_get_ticket_and_csrf(self.server) {
            if let Some(node) = api_get_node(self.server, &ticket) {
                let res = if c.kind == "vm" { "qemu" } else { "lxc" };
                api_post(self.server, &ticket, &csrf,
                    &format!("/nodes/{node}/{res}/{}/status/start", c.vmid));
            }
        }
    }
    fn stop(&self, c: &Container) {
        if let Some((ticket, csrf)) = api_get_ticket_and_csrf(self.server) {
            if let Some(node) = api_get_node(self.server, &ticket) {
                let res = if c.kind == "vm" { "qemu" } else { "lxc" };
                api_post(self.server, &ticket, &csrf,
                    &format!("/nodes/{node}/{res}/{}/status/stop", c.vmid));
            }
        }
    }
    /// API-only 서버는 콘솔 세션을 직접 제공할 수 없음.
    fn console(&self, _c: &Container) -> Option<String> {
        None
    }
}

pub(crate) fn client_for(server: &ProxmoxServer) -> Box<dyn ProxmoxClient + '_> {
    match server.access {
        AccessType::Ssh => Box::new(SshProxmoxClient { server }),
        AccessType::Api => Box::new(ApiProxmoxClient { server }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_cmd_localhost_strips_ssh() {
        let s = ProxmoxServer {
            name: "local".into(), host: "127.0.0.1".into(), user: "root".into(),
            access: AccessType::Ssh, password: None, port: 8006,
        };
        let c = Container {
            vmid: 200, name: "n".into(), status: "running".into(), kind: "lxc".into(),
        };
        let cmd = client_for(&s).console(&c).unwrap();
        assert!(!cmd.starts_with("ssh "), "localhost must not use ssh wrapper: {cmd}");
        assert!(cmd.contains("pct enter 200"));
    }

    #[test]
    fn console_cmd_remote_uses_ssh() {
        let s = ProxmoxServer {
            name: "peer".into(), host: "192.0.2.99".into(), user: "root".into(),
            access: AccessType::Ssh, password: None, port: 8006,
        };
        let c = Container {
            vmid: 100, name: "n".into(), status: "running".into(), kind: "vm".into(),
        };
        let cmd = client_for(&s).console(&c).unwrap();
        assert!(cmd.starts_with("ssh -t root@192.0.2.99"));
        assert!(cmd.contains("qm terminal 100"));
    }

    #[test]
    fn api_console_is_none() {
        let s = ProxmoxServer {
            name: "api".into(), host: "10.0.0.1".into(), user: "root".into(),
            access: AccessType::Api, password: None, port: 8006,
        };
        let c = Container {
            vmid: 1, name: "n".into(), status: "running".into(), kind: "lxc".into(),
        };
        assert!(client_for(&s).console(&c).is_none());
    }
}
