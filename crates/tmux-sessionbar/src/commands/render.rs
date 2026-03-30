use crate::config::template::load_config;
use anyhow::{Result, bail};
use tmux_fmt::tmux;
use tmux_fmt::{Line, click, fallback_window_list, label, styled};

pub fn run(segment: &str) -> Result<()> {
    match segment {
        "left" => render_left(),
        "right" => render_right(),
        _ => bail!("unknown segment: {segment}"),
    }
}

fn render_left() -> Result<()> {
    if !tmux::acquire_guard("sessionbar_render", 100) {
        return Ok(());
    }

    let config = load_config()?;
    let sl = &config.blocks.session_list;
    let th = &config.theme;

    let current = tmux::query_or(&["display-message", "-p", "#S"], "");
    let view_user = tmux::query_or(&["show", "-gv", "@view_user"], "");

    let sessions = tmux::lines(&["list-sessions", "-F", "#{session_name}"])?;

    let mut parts = Vec::new();
    for name in &sessions {
        if !tmux::should_show_for_user(name, &view_user) {
            continue;
        }
        let mut block = if *name == current {
            click(
                name,
                &sl.active_fg,
                &sl.active_bg,
                true,
                &format!(" {name} "),
            )
        } else {
            click(
                name,
                &sl.inactive_fg,
                &sl.inactive_bg,
                false,
                &format!(" {name} "),
            )
        };

        if sl.show_kill_button {
            block.push_str(&click(
                &format!("_k{name}"),
                &sl.kill_fg,
                &config.status.bg,
                false,
                " x ",
            ));
        }

        parts.push(block);
    }

    let mut session_blocks = parts.join(&sl.separator);

    if sl.show_new_button {
        session_blocks.push_str(&format!(
            " {}",
            click("_new_", &sl.button_fg, &sl.button_bg, false, " + ")
        ));
    }

    let sys_stats = get_system_stats(th);

    let mut right_parts = Vec::new();
    for block in &config.status.right.blocks {
        match block.as_str() {
            "hostname" => right_parts.push(styled(
                &config.blocks.hostname.fg,
                &config.blocks.hostname.bg,
                &config.blocks.hostname.format,
            )),
            "datetime" => right_parts.push(styled(
                &config.blocks.datetime.fg,
                &config.blocks.datetime.bg,
                &config.blocks.datetime.format,
            )),
            _ => {}
        }
    }

    let clear_btn = if config.keybindings.pane_clear {
        format!(
            " {}",
            click("_clear_", &sl.active_fg, &th.clear_bg, false, " \u{1f9f9} ")
        )
    } else {
        String::new()
    };

    let right_content = format!("{sys_stats}{}", right_parts.join(""));

    let view_switcher = std::process::Command::new("tmux-windowbar")
        .args(["render-view"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let window_section = std::process::Command::new("tmux-windowbar")
        .args(["render"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

    let session_label = label("Sessions", &th.label_fg);
    let right_section = format!("{right_content} {view_switcher}{clear_btn}");
    let format = if let Some(windows) = window_section {
        Line::new()
            .left()
            .push(&session_label)
            .push(&session_blocks)
            .push(" ")
            .push(&windows)
            .right()
            .push(&right_section)
            .build()
    } else {
        let left = format!("{session_label}{session_blocks}");
        fallback_window_list(&left, &right_section)
    };

    tmux::run(&["set", "-g", "status-format[1]", &format])?;

    Ok(())
}

fn render_right() -> Result<()> {
    Ok(())
}

use crate::config::template::ThemeConfig;

fn get_system_stats(th: &ThemeConfig) -> String {
    let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let cpu_load = load.split_whitespace().next().unwrap_or("0");

    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            avail_kb = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
        }
    }
    let used_gb = (total_kb - avail_kb) as f64 / 1048576.0;
    let total_gb = total_kb as f64 / 1048576.0;
    let mem_pct = if total_kb > 0 {
        (total_kb - avail_kb) * 100 / total_kb
    } else {
        0
    };

    let mem_color = if mem_pct > 80 {
        &th.mem_critical
    } else if mem_pct > 60 {
        &th.mem_warn
    } else {
        &th.mem_normal
    };

    format!(
        "{}{}",
        styled(&th.stats_fg, &th.stats_bg, &format!(" {cpu_load} ")),
        styled(
            &th.mem_fg,
            mem_color,
            &format!(" {used_gb:.1}/{total_gb:.0}G ")
        ),
    )
}
