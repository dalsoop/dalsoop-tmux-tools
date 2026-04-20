//! Proxmox REST API(curl 기반) 헬퍼. API-only 모드에서만 사용.

use std::process::Command;

use super::parse::extract_json_str;
use super::types::{Container, ProxmoxServer};

pub(crate) fn api_get_ticket(server: &ProxmoxServer) -> Option<String> {
    let url = format!("https://{}:{}/api2/json/access/ticket", server.host, server.port);
    let user = format!("{}@pam", server.user);
    let password = server.password.as_deref().unwrap_or("");
    let output = Command::new("curl")
        .args(["-sk", "--connect-timeout", "5", "-d",
               &format!("username={user}&password={password}"), &url])
        .output().ok()?;
    if !output.status.success() { return None; }
    let text = String::from_utf8_lossy(&output.stdout);
    let ticket_start = text.find("\"ticket\":\"")?;
    let rest = &text[ticket_start + 10..];
    let ticket_end = rest.find('"')?;
    Some(rest[..ticket_end].to_string())
}

pub(crate) fn api_get(server: &ProxmoxServer, ticket: &str, path: &str) -> Option<String> {
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

pub(crate) fn api_post(server: &ProxmoxServer, ticket: &str, csrf: &str, path: &str) -> bool {
    let url = format!("https://{}:{}/api2/json{path}", server.host, server.port);
    let cookie = format!("PVEAuthCookie={ticket}");
    Command::new("curl")
        .args(["-sk", "--connect-timeout", "10", "-X", "POST",
               "-b", &cookie, "-H", &format!("CSRFPreventionToken: {csrf}"), &url])
        .status().map(|s| s.success()).unwrap_or(false)
}

pub(crate) fn api_get_ticket_and_csrf(server: &ProxmoxServer) -> Option<(String, String)> {
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

pub(crate) fn api_get_node(server: &ProxmoxServer, ticket: &str) -> Option<String> {
    let text = api_get(server, ticket, "/nodes")?;
    let start = text.find("\"node\":\"")?;
    let rest = &text[start + 8..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

pub(crate) fn api_fetch_containers(server: &ProxmoxServer) -> Vec<Container> {
    let ticket = match api_get_ticket(server) {
        Some(t) => t, None => return Vec::new(),
    };
    let node = match api_get_node(server, &ticket) {
        Some(n) => n, None => return Vec::new(),
    };

    let mut result = Vec::new();

    if let Some(text) = api_get(server, &ticket, &format!("/nodes/{node}/lxc")) {
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
