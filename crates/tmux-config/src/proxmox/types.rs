//! Proxmox 도메인의 데이터 타입 집합.

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

/// 컨테이너 상세 패널에 한번에 필요한 정보.
#[derive(Clone, Debug)]
pub struct DetailInfo {
    pub resources: Option<Resources>,
    pub docker: Vec<DockerContainer>,
    pub ports: Vec<PortInfo>,
    pub snapshots: Vec<Snapshot>,
}

#[derive(Clone, Debug)]
pub struct PortInfo {
    pub addr: String,
    pub port: u16,
    pub process: String,
    pub proto_guess: String, // http, https, ssh, ws, db, etc.
}

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
