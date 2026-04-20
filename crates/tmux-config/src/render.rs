use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use crate::{
    App, Mode,
    BG, FG, GREEN, BLUE, RED, SUBTLE, BORDER,
    apps, dal, proxmox, seed, ssh, tabs,
};
use tabs::Tab;

pub(crate) fn render(f: &mut ratatui::Frame, app: &mut App) {
    let full = f.area();

    // Outer border block
    let outer = Block::default()
        .title(Span::styled(" tmux-topbar ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));
    let inner = outer.inner(full);
    f.render_widget(outer, full);

    // Split: tab bar (3) | body (fill) | hint bar (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // Tab bar
    tabs::render_tab_bar(f, app.tab, chunks[0]);

    // Body area (may be split for form overlay)
    render_body(f, app, chunks[1]);

    // Hint / status bar
    render_hint(f, app, chunks[2]);
}

pub(crate) fn render_body(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    match &app.mode {
        Mode::Normal => render_list(f, app, area),
        Mode::Confirming { label, .. } => {
            let label = label.clone();
            render_list(f, app, area);
            render_confirm_overlay(f, &label, area, app.tab == Tab::Proxmox);
        }
        Mode::Editing => {
            let form_height = {
                let fields = app.form.as_ref().map_or(0, |f| f.fields.len());
                (fields as u16 + 4).min(area.height.saturating_sub(3))
            };
            let list_height = area.height.saturating_sub(form_height);
            let parts = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(list_height), Constraint::Length(form_height)])
                .split(area);
            render_list(f, app, parts[0]);
            render_form(f, app, parts[1]);
        }
        Mode::SeedBrowse { list } => {
            let items: Vec<ListItem> = seed::SEEDS.iter()
                .map(|s| {
                    let installed = seed::is_installed(s.command);
                    let in_config = app.config.apps.iter().any(|a| a.command == s.command);
                    let method = seed::install_method(s);
                    let status = if in_config {
                        "✓ added".to_string()
                    } else if installed {
                        "✓ ready".to_string()
                    } else if method != "-" {
                        format!("✗ via {method}")
                    } else {
                        "✗ n/a".to_string()
                    };
                    let style = if in_config {
                        Style::default().fg(SUBTLE)
                    } else if installed {
                        Style::default().fg(GREEN)
                    } else if method != "-" {
                        Style::default().fg(FG)
                    } else {
                        Style::default().fg(RED)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("  {} {:<14} ", s.emoji, s.command), style),
                        Span::styled(format!("{:<12} ", status), if installed || in_config { Style::default().fg(GREEN) } else if method != "-" { Style::default().fg(BLUE) } else { Style::default().fg(RED) }),
                        Span::styled(s.description, Style::default().fg(SUBTLE)),
                    ]))
                })
                .collect();

            let mut state = list.state.clone();
            let widget = List::new(items)
                .block(
                    Block::default()
                        .title(Span::styled(" Install Apps ", Style::default().fg(GREEN)))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(BLUE))
                        .style(Style::default().bg(BG)),
                )
                .style(Style::default().fg(FG).bg(BG))
                .highlight_style(Style::default().fg(BLUE).bg(Color::Rgb(58, 63, 76)).add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            f.render_stateful_widget(widget, area, &mut state);
        }
    }
}

pub(crate) fn render_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Proxmox => render_proxmox_list(f, app, area),
        Tab::Dal => render_dal_list(f, app, area),
        _ => render_standard_list(f, app, area),
    }
}

pub(crate) fn render_standard_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let (title, items, list_view) = match app.tab {
        Tab::Ssh => {
            let items: Vec<ListItem> = app.config.ssh.iter()
                .map(|e| ListItem::new(ssh::display(e)))
                .collect();
            ("SSH Hosts", items, &mut app.ssh)
        }
        Tab::Apps => {
            let items: Vec<ListItem> = app.config.apps.iter()
                .map(|a| ListItem::new(apps::display(a)))
                .collect();
            ("Apps", items, &mut app.apps)
        }
        Tab::Settings => {
            let items: Vec<ListItem> = app.setting_items.iter()
                .map(|s| {
                    if s.source == crate::settings::SettingSource::Header {
                        ListItem::new(Line::from(Span::styled(
                            s.label,
                            Style::default().fg(BLUE).add_modifier(Modifier::BOLD),
                        )))
                    } else {
                        ListItem::new(format!("  {:28}  {}", s.label, s.value))
                    }
                })
                .collect();
            ("Settings", items, &mut app.settings)
        }
        Tab::Proxmox | Tab::Dal => unreachable!(),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(format!(" {title} "), Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().fg(FG).bg(BG))
        .highlight_style(
            Style::default()
                .fg(BLUE)
                .bg(Color::Rgb(58, 63, 76))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut list_view.state);
}

pub(crate) fn render_proxmox_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let breadcrumb = app.pve_breadcrumb();

    // Track how many non-selectable header lines precede the selectable items
    let mut header_lines: usize = 0;

    let items: Vec<ListItem> = match app.pve_depth {
        0 => app.pve_servers.iter()
            .map(|s| ListItem::new(proxmox::display_server(s)))
            .collect(),
        1 => {
            let mut items: Vec<ListItem> = Vec::new();
            if let Some(hi) = &app.pve_host_info {
                for line in proxmox::display_host_info(hi) {
                    items.push(ListItem::new(line));
                }
                header_lines = items.len();
            }
            for c in &app.pve_containers {
                items.push(ListItem::new(proxmox::display_container(c)));
            }
            items
        }
        _ => {
            if let Some(detail) = &app.pve_detail {
                proxmox::display_detail(detail).into_iter()
                    .map(ListItem::new)
                    .collect()
            } else {
                vec![ListItem::new("  (no data)")]
            }
        }
    };

    // Offset the ListState selection by header_lines for rendering
    let mut render_state = app.pve_list.state.clone();
    if header_lines > 0 {
        if let Some(sel) = render_state.selected() {
            render_state.select(Some(sel + header_lines));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(format!(" {breadcrumb} "), Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().fg(FG).bg(BG))
        .highlight_style(
            Style::default()
                .fg(BLUE)
                .bg(Color::Rgb(58, 63, 76))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut render_state);
}

pub(crate) fn render_dal_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let yellow = Color::Rgb(229, 192, 123);

    let tester_status = if app.dal.tester_alive { "●" } else { "○" };
    let running = app.dal.count_running();
    let running_str = if running > 0 { format!("  {running} running") } else { String::new() };
    let summary = format!(
        " Dal {tester_status} — {} pending{running_str}  {} passed  {} failed ",
        app.dal.count_pending(),
        app.dal.count_passed(),
        app.dal.count_failed(),
    );

    let items: Vec<ListItem> = app.dal.queue.iter().map(|t| {
        let (icon, color) = match t.status {
            dal::TestStatus::Pending  => ("○", SUBTLE),
            dal::TestStatus::Running  => ("◉", yellow),
            dal::TestStatus::Passed   => ("✓", GREEN),
            dal::TestStatus::Failed   => ("✗", RED),
        };
        let duration = if t.duration_ms > 0 {
            format!(" ({:.1}s)", t.duration_ms as f64 / 1000.0)
        } else if t.status == dal::TestStatus::Running {
            if let Some(start) = t.submitted_at {
                format!(" ({:.0}s...)", start.elapsed().as_secs_f64())
            } else {
                " (...)".into()
            }
        } else {
            String::new()
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {icon} "), Style::default().fg(color)),
            Span::styled(format!("{:<24}", t.label), Style::default().fg(FG)),
            Span::styled(&t.trigger, Style::default().fg(SUBTLE)),
            Span::styled(duration, Style::default().fg(SUBTLE)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(summary, Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().fg(FG).bg(BG))
        .highlight_style(
            Style::default()
                .fg(BLUE)
                .bg(Color::Rgb(58, 63, 76))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.dal_list.state);
}

pub(crate) fn render_form(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let form = match &app.form {
        Some(fm) => fm,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        let cursor = if i == form.focused { "\u{2588}" } else { "" };
        let label_style = if i == form.focused {
            Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SUBTLE)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:20} ", field.label), label_style),
            Span::styled(field.value.clone(), Style::default().fg(FG)),
            Span::styled(cursor, Style::default().fg(BLUE)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [Enter]", Style::default().fg(GREEN)),
        Span::raw(" Save  "),
        Span::styled("[Esc]", Style::default().fg(RED)),
        Span::raw(" Cancel  "),
        Span::styled("[Tab]", Style::default().fg(SUBTLE)),
        Span::raw(" Next field"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" Edit ", Style::default().fg(BLUE)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BLUE))
                .style(Style::default().bg(BG)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

pub(crate) fn render_confirm_overlay(f: &mut ratatui::Frame, label: &str, area: Rect, is_stop: bool) {
    // Center a small box in the area
    let w = 50u16.min(area.width.saturating_sub(4));
    let h = 5u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect::new(x, y, w, h);

    let action = if is_stop { "Stop" } else { "Delete" };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(format!("  {action} '")),
            Span::styled(label, Style::default().fg(RED)),
            Span::raw("' ?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y]", Style::default().fg(RED)),
            Span::raw(" Yes  "),
            Span::styled("[n/Esc]", Style::default().fg(GREEN)),
            Span::raw(" No"),
        ]),
    ];

    let title = if is_stop { " Confirm Stop " } else { " Confirm Delete " };
    let para = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(title, Style::default().fg(RED)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(RED))
            .style(Style::default().bg(BG)),
    );
    f.render_widget(para, rect);
}

pub(crate) fn render_hint(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let text = if let Some(msg) = &app.status_msg {
        Line::from(Span::styled(msg.clone(), Style::default().fg(GREEN)))
    } else {
        match app.mode {
            Mode::Normal => {
                if app.tab == Tab::Proxmox {
                    let mut spans = Vec::new();
                    if app.pve_depth == 0 {
                        spans.extend([
                            Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" open  "),
                            Span::styled("[c]", Style::default().fg(GREEN)), Span::raw("onnect  "),
                            Span::styled("[k]", Style::default().fg(BLUE)), Span::raw("ey install  "),
                        ]);
                    } else if app.pve_depth == 1 {
                        spans.extend([
                            Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" detail  "),
                            Span::styled("[c]", Style::default().fg(GREEN)), Span::raw("onsole  "),
                            Span::styled("[l]", Style::default().fg(BLUE)), Span::raw("ogs  "),
                            Span::styled("[s]", Style::default().fg(GREEN)), Span::raw("tart  "),
                            Span::styled("[x]", Style::default().fg(RED)), Span::raw(" stop  "),
                            Span::styled("[D]", Style::default().fg(RED)), Span::raw("el  "),
                        ]);
                    } else if app.pve_depth == 2 {
                        spans.extend([
                            Span::styled("[l]", Style::default().fg(BLUE)), Span::raw("ogs  "),
                            Span::styled("[e]", Style::default().fg(GREEN)), Span::raw("xec  "),
                            Span::styled("[s]", Style::default().fg(GREEN)), Span::raw("tart  "),
                            Span::styled("[x]", Style::default().fg(RED)), Span::raw(" stop  "),
                        ]);
                    }
                    if app.pve_depth > 0 {
                        spans.extend([Span::styled("[←/Esc]", Style::default().fg(SUBTLE)), Span::raw(" back  ")]);
                    }
                    spans.extend([
                        Span::styled("[r]", Style::default().fg(BLUE)), Span::raw("efresh  "),
                        Span::styled("[Tab]", Style::default().fg(SUBTLE)), Span::raw(" tab  "),
                        Span::styled("[q]", Style::default().fg(SUBTLE)), Span::raw("uit"),
                    ]);
                    Line::from(spans)
                } else if app.tab == Tab::Dal {
                    Line::from(vec![
                        Span::styled("[w]", Style::default().fg(GREEN)), Span::raw("ake  "),
                        Span::styled("[W]", Style::default().fg(RED)), Span::raw(" sleep  "),
                        Span::styled("[s]", Style::default().fg(BLUE)), Span::raw("can  "),
                        Span::styled("[r/Enter]", Style::default().fg(GREEN)), Span::raw(" submit  "),
                        Span::styled("[a]", Style::default().fg(BLUE)), Span::raw("ll  "),
                        Span::styled("[A]", Style::default().fg(BLUE)), Span::raw("ll+run  "),
                        Span::styled("[c]", Style::default().fg(SUBTLE)), Span::raw("lear  "),
                        Span::styled("[q]", Style::default().fg(SUBTLE)), Span::raw("uit"),
                    ])
                } else {
                    let mut spans = vec![
                        Span::styled("[a]", Style::default().fg(BLUE)), Span::raw("dd  "),
                        Span::styled("[e]", Style::default().fg(BLUE)), Span::raw("dit  "),
                        Span::styled("[d]", Style::default().fg(RED)),  Span::raw("elete  "),
                    ];
                    if app.tab == Tab::Ssh {
                        spans.push(Span::styled("[c]", Style::default().fg(GREEN)));
                        spans.push(Span::raw("onnect  "));
                    }
                    if app.tab == Tab::Apps {
                        spans.push(Span::styled("[i]", Style::default().fg(GREEN)));
                        spans.push(Span::raw("nstall  "));
                    }
                    spans.push(Span::styled("[Tab]", Style::default().fg(SUBTLE)));
                    spans.push(Span::raw(" switch tab  "));
                    spans.push(Span::styled("[q]", Style::default().fg(SUBTLE)));
                    spans.push(Span::raw("uit"));
                    Line::from(spans)
                }
            }
            Mode::SeedBrowse { .. } => Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" install & add  "),
                Span::styled("[Esc]", Style::default().fg(RED)), Span::raw(" back"),
            ]),
            Mode::Editing => Line::from(vec![
                Span::styled("[Tab]", Style::default().fg(BLUE)), Span::raw(" next field  "),
                Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" save  "),
                Span::styled("[Esc]", Style::default().fg(RED)), Span::raw(" cancel"),
            ]),
            Mode::Confirming { .. } => Line::from(vec![
                Span::styled("[y]", Style::default().fg(RED)), Span::raw(" confirm  "),
                Span::styled("[n/Esc]", Style::default().fg(GREEN)), Span::raw(" cancel"),
            ]),
        }
    };
    let para = Paragraph::new(text)
        .style(Style::default().fg(FG).bg(BG));
    f.render_widget(para, area);
}
