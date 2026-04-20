//! ratatui Line/Span 생성 — 리스트 / 상세 / 호스트 요약 뷰.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use super::ssh::is_localhost;
use super::types::{AccessType, Container, DetailInfo, HostInfo, ProxmoxServer};

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

/// 상세(depth 2) 패널 전체를 Line 벡터로 구성.
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

    // Docker
    lines.push(Line::from(Span::styled(
        format!("  🐳 Docker ({})", info.docker.len()), header
    )));
    if info.docker.is_empty() {
        lines.push(Line::from(Span::styled("     (none or no docker)", dim)));
    } else {
        for d in &info.docker {
            let color = if d.status.starts_with("Up") { green } else { red };
            lines.push(Line::from(vec![
                Span::styled(format!("     {:<20}", d.name), color),
                Span::styled(format!(" {:<30}", d.image), dim),
                Span::styled(d.status.clone(), val),
            ]));
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

    // Listening Ports
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
