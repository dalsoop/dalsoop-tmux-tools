use crate::config::template::load_config;
use std::process::Command;

pub fn run(segment: &str) -> Result<(), Box<dyn std::error::Error>> {
    match segment {
        "left" => render_left(),
        "right" => render_right(),
        _ => Err(format!("unknown segment: {segment}").into()),
    }
}

fn render_left() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let sl = &config.blocks.session_list;

    // Get current client session
    let current = Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // Get view user filter
    let view_user = Command::new("tmux")
        .args(["show", "-gv", "@view_user"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // List all sessions
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()?;
    let sessions = String::from_utf8_lossy(&output.stdout);

    let mut parts = Vec::new();
    for name in sessions.lines() {
        if name.is_empty() {
            continue;
        }
        // Filter by user if set
        // Sessions named after a user belong to that user
        // Sessions with other names (numbers, etc) belong to root
        if !view_user.is_empty() {
            let is_user_session = name == view_user;
            let is_unowned = !name.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false);
            let belongs_to_root = is_unowned && view_user == "root";
            if !is_user_session && !belongs_to_root {
                continue;
            }
        }
        let mut block = if name == current {
            format!(
                "#[range=user|{name}]#[fg={},bg={},bold] {name} #[norange]",
                sl.active_fg, sl.active_bg,
            )
        } else {
            format!(
                "#[range=user|{name}]#[fg={},bg={}] {name} #[norange]",
                sl.inactive_fg, sl.inactive_bg,
            )
        };

        // [x] kill button — hide for current session
        if sl.show_kill_button && name != current {
            block.push_str(&format!(
                "#[range=user|_k{name}]#[fg={},bg=#282c34] x #[norange default]",
                sl.kill_bg,
            ));
        } else {
            block.push_str("#[default]");
        }

        parts.push(block);
    }

    let mut session_blocks = parts.join(&sl.separator);

    // [+] new session button
    if sl.show_new_button {
        session_blocks.push_str(&format!(
            " #[range=user|_new_]#[fg={},bg={}] + #[norange default]",
            sl.button_fg, sl.button_bg,
        ));
    }

    // System stats
    let sys_stats = get_system_stats();

    // Build right side from config
    let mut right_parts = Vec::new();
    for block in &config.status.right.blocks {
        match block.as_str() {
            "hostname" => {
                right_parts.push(format!(
                    "#[fg={},bg={}]{}",
                    config.blocks.hostname.fg,
                    config.blocks.hostname.bg,
                    config.blocks.hostname.format
                ));
            }
            "datetime" => {
                right_parts.push(format!(
                    "#[fg={},bg={}]{}",
                    config.blocks.datetime.fg,
                    config.blocks.datetime.bg,
                    config.blocks.datetime.format
                ));
            }
            _ => {}
        }
    }
    let right_content = format!("{sys_stats}{}", right_parts.join(""));

    // Get view switcher from windowbar
    let view_switcher = Command::new("tmux-windowbar")
        .args(["render-view"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Try to get window list from tmux-windowbar
    let window_section = Command::new("tmux-windowbar")
        .args(["render"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

    let session_label = "#[fg=#98c379,bold]Sessions #[default]";
    let format = if let Some(windows) = window_section {
        format!(
            "#[align=left default]{session_label}{session_blocks} \
             {windows}\
             #[align=right default]{right_content} {view_switcher}"
        )
    } else {
        format!(
            "#[align=left default]{session_label}{session_blocks}\
             #[list=on align=left]\
             #[list=left-marker]<#[list=right-marker]>\
             #[list=on]\
             #{{W:\
             #[range=window|#{{window_index}} #{{E:window-status-style}}]\
             #[push-default]#{{T:window-status-format}}#[pop-default]\
             #[norange default]#{{?window_end_flag,,#{{window-status-separator}}}},\
             #[range=window|#{{window_index}} list=focus #{{E:window-status-current-style}}]\
             #[push-default]#{{T:window-status-current-format}}#[pop-default]\
             #[norange list=on default]#{{?window_end_flag,,#{{window-status-separator}}}}\
             }}\
             #[nolist align=right default]{right_content} {view_switcher}"
        )
    };

    // Always index 1
    Command::new("tmux")
        .args(["set", "-g", "status-format[1]", &format])
        .status()?;

    Ok(())
}

fn render_right() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn get_system_stats() -> String {
    // CPU usage from /proc/loadavg
    let load = std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_default();
    let cpu_load = load.split_whitespace().next().unwrap_or("0");

    // Memory from /proc/meminfo
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            avail_kb = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    let used_gb = (total_kb - avail_kb) as f64 / 1048576.0;
    let total_gb = total_kb as f64 / 1048576.0;
    let mem_pct = if total_kb > 0 { ((total_kb - avail_kb) * 100 / total_kb) as u64 } else { 0 };

    let mem_color = if mem_pct > 80 { "#e06c75" } else if mem_pct > 60 { "#e5c07b" } else { "#98c379" };

    format!(
        "#[fg=#abb2bf,bg=#3e4452] {cpu_load} #[fg=#282c34,bg={mem_color}] {used_gb:.1}/{total_gb:.0}G ",
    )
}
