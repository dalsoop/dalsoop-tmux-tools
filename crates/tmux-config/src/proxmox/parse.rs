//! sh/ssh 명령 출력을 구조체로 변환하는 순수 파서 모음.
//!
//! 입력은 문자열(String/&str) 뿐, 외부 I/O 없음 — 단위 테스트에 적합.

use super::types::{
    Container, DockerContainer, HostInfo, PortInfo, Resources, Snapshot,
};

/// JSON 문자열에서 `"key":"value"` 의 value 를 스칼라로 뽑는 얇은 헬퍼
/// (완전한 파서 아님 — Proxmox ticket/CSRF 응답에만 사용).
pub fn extract_json_str(text: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":\"");
    let start = text.find(&pattern)?;
    let rest = &text[start + pattern.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 포트 번호·프로세스 이름 기준으로 추정 프로토콜 라벨.
pub fn guess_protocol(port: u16, process: &str) -> String {
    let p = process.to_lowercase();
    if p.contains("nginx") || p.contains("apache") || p.contains("caddy") || p.contains("traefik") {
        if port == 443 || port == 8443 { return "https".into(); }
        return "http".into();
    }
    if p.contains("sshd") { return "ssh".into(); }
    if p.contains("postgres") { return "postgresql".into(); }
    if p.contains("mysql") || p.contains("mariadbd") { return "mysql".into(); }
    if p.contains("redis") { return "redis".into(); }
    if p.contains("mongo") { return "mongodb".into(); }
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

/// `pct list` 출력 한 덩어리를 Container 벡터로.
pub fn parse_pct_list(out: &str) -> Vec<Container> {
    out.lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 3 {
                Some(Container {
                    vmid: cols[0].parse().unwrap_or(0),
                    status: cols[1].to_string(),
                    name: cols.last().unwrap_or(&"").to_string(),
                    kind: "lxc".into(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// `qm list` 출력 파싱.
pub fn parse_qm_list(out: &str) -> Vec<Container> {
    out.lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 3 {
                Some(Container {
                    vmid: cols[0].parse().unwrap_or(0),
                    name: cols[1].to_string(),
                    status: cols[2].to_lowercase(),
                    kind: "vm".into(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// `docker ps --format 'Names\tImage\tStatus\tPorts'` 출력.
pub fn parse_docker(out: &str) -> Vec<DockerContainer> {
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

/// `ss -tlnp | tail -n+2` 출력 파싱.
pub fn parse_ports(out: &str) -> Vec<PortInfo> {
    out.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            let local = cols.get(3)?;
            let process = cols.get(5..)
                .map(|rest| rest.join(" "))
                .and_then(|s| {
                    let start = s.find("((\"")? + 3;
                    let end = s[start..].find('"')? + start;
                    Some(s[start..end].to_string())
                })
                .unwrap_or_default();
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

/// `qm/pct status + config` 혼합 출력 파싱. IP 는 별도 호출에서 주입.
pub fn parse_resources(out: &str) -> Resources {
    let mut cpus = "?".to_string();
    let mut mem_total = "?".to_string();
    let mut disk_total = "?".to_string();
    let mut mem_used = "?".to_string();
    let mut uptime = "?".to_string();

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

    Resources {
        cpus, mem_used, mem_total,
        _disk_used: "-".into(), disk_total, uptime,
        ip: "-".into(),
    }
}

/// `pct/qm listsnapshot` 출력 파싱.
pub fn parse_snapshots(out: &str) -> Vec<Snapshot> {
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

/// `hostname:<v>\nkernel:<v>\n...` 한 줄씩 붙은 출력을 구조체로.
pub fn parse_host_info(out: &str) -> HostInfo {
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
    info
}

/// `/etc/pve/corosync.conf` 본문에서 `(name, ring0_addr)` pair 추출.
pub fn parse_cluster_members(text: &str) -> Vec<(String, String)> {
    let mut members = Vec::new();
    let mut cur_name: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("name:") {
            cur_name = Some(rest.trim().to_string());
        } else if let Some(rest) = t.strip_prefix("ring0_addr:") {
            if let Some(name) = cur_name.take() {
                members.push((name, rest.trim().to_string()));
            }
        }
    }
    members
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cluster_members_basic() {
        let sample = r#"
nodelist {
  node {
    name: pve
    nodeid: 1
    ring0_addr: 192.168.2.50
  }
  node {
    name: ranode-3960x
    nodeid: 2
    ring0_addr: 192.168.2.60
  }
}
"#;
        let m = parse_cluster_members(sample);
        assert_eq!(m, vec![
            ("pve".into(), "192.168.2.50".into()),
            ("ranode-3960x".into(), "192.168.2.60".into()),
        ]);
    }

    #[test]
    fn parse_cluster_members_skips_lone_cluster_name() {
        assert!(parse_cluster_members("cluster_name: dal\n").is_empty());
    }

    #[test]
    fn parse_pct_list_two_rows() {
        let out = "VMID       Status     Lock         Name\n\
                   100        running                 gitlab\n\
                   200        stopped                 redis";
        let r = parse_pct_list(out);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].vmid, 100);
        assert_eq!(r[0].name, "gitlab");
        assert_eq!(r[0].status, "running");
        assert_eq!(r[1].status, "stopped");
    }

    #[test]
    fn parse_qm_list_two_rows() {
        let out = "      VMID NAME                 STATUS     MEM(MB)\n\
                   50055 linuxmint            running    2048\n\
                   50099 posthog              stopped    1024";
        let r = parse_qm_list(out);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].vmid, 50055);
        assert_eq!(r[0].kind, "vm");
        assert_eq!(r[1].status, "stopped");
    }

    #[test]
    fn parse_host_info_fields() {
        let out = "hostname:pve\nkernel:6.8\ncpu:Threadripper\ncores:32\nmem:100MB/504000MB\nuptime:up 2 days\nload:1 2 3\npve:8.x";
        let h = parse_host_info(out);
        assert_eq!(h.hostname, "pve");
        assert_eq!(h.cpu_cores, "32");
        assert_eq!(h.mem_used, "100MB");
        assert_eq!(h.mem_total, "504000MB");
    }

    #[test]
    fn parse_docker_one() {
        let out = "web\tnginx:1\tUp 2d\t0.0.0.0:80->80/tcp";
        let d = parse_docker(out);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].name, "web");
        assert_eq!(d[0].image, "nginx:1");
    }

    #[test]
    fn guess_protocol_http_on_nginx() {
        assert_eq!(guess_protocol(8080, "nginx"), "http");
        assert_eq!(guess_protocol(443, "nginx"), "https");
        assert_eq!(guess_protocol(22, "sshd"), "ssh");
        assert_eq!(guess_protocol(7777, ""), "tcp");
    }

    #[test]
    fn extract_json_str_basic() {
        let s = r#"{"ticket":"abc","CSRFPreventionToken":"xyz"}"#;
        assert_eq!(extract_json_str(s, "ticket").as_deref(), Some("abc"));
        assert_eq!(extract_json_str(s, "missing"), None);
    }

    // ── Fixture 회귀 테스트 ──
    // 실제 호스트에서 캡처한 pct/qm/corosync 출력. 포맷이 바뀌면 여기서 먼저 깨짐.

    const PCT_FIXTURE: &str = include_str!("tests/fixtures/pct_list.txt");
    const QM_FIXTURE: &str = include_str!("tests/fixtures/qm_list.txt");
    const COROSYNC_FIXTURE: &str = include_str!("tests/fixtures/corosync.conf");

    #[test]
    fn fixture_pct_list_parses() {
        let rows = parse_pct_list(PCT_FIXTURE);
        assert!(rows.len() >= 3, "expected >=3 rows, got {}", rows.len());
        let first = &rows[0];
        assert!(first.vmid > 0);
        assert_eq!(first.kind, "lxc");
        assert!(!first.name.is_empty());
        assert!(first.status == "running" || first.status == "stopped");
    }

    #[test]
    fn fixture_qm_list_parses() {
        let rows = parse_qm_list(QM_FIXTURE);
        assert!(!rows.is_empty(), "expected >=1 VM row");
        for r in &rows {
            assert!(r.status.chars().all(|c| !c.is_uppercase()), "status lower: {:?}", r.status);
            assert_eq!(r.kind, "vm");
        }
    }

    #[test]
    fn fixture_corosync_extracts_cluster_members() {
        let members = parse_cluster_members(COROSYNC_FIXTURE);
        assert!(members.len() >= 2, "expected >=2 nodes, got {}", members.len());
        for (name, addr) in &members {
            assert!(!name.is_empty());
            assert!(!addr.contains(' '), "addr must be single token: {addr}");
        }
    }
}
