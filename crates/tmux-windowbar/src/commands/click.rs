use crate::config::template::load_config;
use anyhow::{Result, bail};
use tmux_fmt::tmux;

const CONFIRM_FILE: &str = "/tmp/tmux-pending-confirm.conf";
const RENAME_FILE: &str = "/tmp/tmux-pending-rename.conf";

pub fn run(range: &str) -> Result<()> {
    if let Some(mode) = range.strip_prefix("_v") {
        let mode = mode.to_lowercase();
        tmux::run(&["set", "-g", "@view_mode", &mode])?;
        if mode == "all" {
            tmux::run_quiet(&["set", "-gu", "@view_user"]);
        }
        let _ = std::process::Command::new("tmux-sessionbar")
            .args(["render-status", "left"])
            .status();
    } else if let Some(idx_str) = range.strip_prefix("_app") {
        if let Ok(idx) = idx_str.parse::<usize>() {
            let config = load_config()?;
            if let Some(app) = config.apps.get(idx) {
                if app.mode == "pane" {
                    tmux::run(&["split-window", "-h", &app.command])?;
                } else if !switch_to_existing_app(&app.command)? {
                    tmux::run(&["new-window", "-n", &app.command, &app.command])?;
                }
            }
        }
    } else if let Some(user) = range.strip_prefix("_u") {
        tmux::run(&["set", "-g", "@view_user", user])?;

        let has = tmux::run(&["has-session", "-t", &format!("={user}")]).is_ok();

        if has {
            tmux::run(&["switch-client", "-t", &format!("={user}")])?;
        } else {
            tmux::run(&["new-session", "-d", "-s", user, &format!("sudo -iu {user}")])?;
            tmux::run(&["switch-client", "-t", &format!("={user}")])?;
        }
        let _ = std::process::Command::new("tmux-sessionbar")
            .args(["render-status", "left"])
            .status();
    } else if range == "_splith" {
        tmux::run(&["split-window", "-h"])?;
    } else if range == "_splitv" {
        tmux::run(&["split-window", "-v"])?;
    } else if range == "_wnew_" {
        tmux::run(&["new-window"])?;
    } else if let Some(idx) = range.strip_prefix("_wk") {
        kill_window(idx)?;
    } else if let Some(idx) = range.strip_prefix("_ws") {
        tmux::run(&["select-window", "-t", &format!(":{idx}")])?;
    } else if let Some(target) = range.strip_prefix("_wa") {
        if let Some((sess, win)) = target.split_once('.') {
            tmux::run(&["switch-client", "-t", &format!("={sess}:{win}")])?;
        }
    } else if let Some(target) = range.strip_prefix("_wx") {
        if let Some((sess, win)) = target.split_once('.') {
            let kill_cmd = format!("kill-window -t ={sess}:{win}");
            confirm_and_run(&format!("Kill window '{sess}:{win}'?"), &kill_cmd)?;
        }
    } else if let Some(target) = range.strip_prefix("_px") {
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            let kill_cmd = format!("kill-pane -t ={sess}:{win}.{pane}");
            confirm_and_run(&format!("Kill pane '{sess}.{win}.{pane}'?"), &kill_cmd)?;
        }
    } else if let Some(target) = range.strip_prefix("_pp") {
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            tmux::run(&["switch-client", "-t", &format!("={sess}:{win}")])?;
            tmux::run(&["select-pane", "-t", &format!("={sess}:{win}.{pane}")])?;
        }
    } else {
        bail!("unknown range: {range}");
    }

    Ok(())
}

pub fn run_dblclick(range: &str) -> Result<()> {
    if range
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        && !range.starts_with('_')
    {
        write_rename_prompt(&format!(
            "command-prompt -I \"{range}\" -p \"rename session:\" \"rename-session '%%'\""
        ))?;
    } else if let Some(idx) = range.strip_prefix("_ws") {
        write_rename_prompt(&format!(
            "command-prompt -p \"rename window {idx}:\" \"rename-window -t :{idx} '%%'\""
        ))?;
    } else if let Some(target) = range.strip_prefix("_wa")
        && let Some((sess, win)) = target.split_once('.')
    {
        write_rename_prompt(&format!(
            "command-prompt -p \"rename window {sess}:{win}:\" \"rename-window -t ={sess}:{win} '%%'\""
        ))?;
    }
    Ok(())
}

fn kill_window(idx: &str) -> Result<()> {
    let windows = tmux::lines(&["list-windows", "-F", "#{window_index}"])?;

    if windows.len() <= 1 {
        tmux::run(&["display-message", "cannot kill last window"])?;
        return Ok(());
    }

    let kill_cmd = format!("kill-window -t :{idx}");
    confirm_and_run(&format!("Kill window '{idx}'?"), &kill_cmd)?;

    Ok(())
}

/// Find an existing window running `command` and switch to it. Returns true if found.
fn switch_to_existing_app(command: &str) -> Result<bool> {
    // List all windows: "session:index:window_name"
    let windows = tmux::lines(&[
        "list-windows",
        "-a",
        "-F",
        "#{session_name}:#{window_index}:#{window_name}",
    ])?;
    for line in &windows {
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() == 3 && parts[2] == command {
            tmux::run(&[
                "switch-client",
                "-t",
                &format!("={}:{}", parts[0], parts[1]),
            ])?;
            return Ok(true);
        }
    }
    Ok(false)
}

fn confirm_and_run(title: &str, cmd: &str) -> Result<()> {
    let safe_title = tmux::sanitize(title);
    let safe_cmd = tmux::sanitize(cmd);
    let content = format!("confirm-before -p \"{safe_title} (y/n)\" \"{safe_cmd}\"");
    std::fs::write(CONFIRM_FILE, content)?;
    Ok(())
}

fn write_rename_prompt(content: &str) -> Result<()> {
    std::fs::write(RENAME_FILE, content)?;
    Ok(())
}
