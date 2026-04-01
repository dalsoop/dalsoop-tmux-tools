use crate::config::template::load_config;
use anyhow::{Result, bail};
use tmux_fmt::tmux;


pub fn run(range: &str) -> Result<()> {
    if let Some(idx_str) = range.strip_prefix("_app") {
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
    } else if let Some(idx_str) = range.strip_prefix("_ssh") {
        if let Ok(idx) = idx_str.parse::<usize>() {
            let config = load_config()?;
            if let Some(entry) = config.ssh.get(idx) {
                let session_name = format!("ssh-{}", entry.name);
                let ssh_target = if let Some(ref user) = entry.user {
                    format!("{user}@{}", entry.host)
                } else {
                    entry.host.clone()
                };

                let has = tmux::run(&["has-session", "-t", &format!("={session_name}")]).is_ok();
                if has {
                    tmux::switch_client(&format!("={session_name}"))?;
                } else {
                    // Auto-reconnecting SSH session
                    let ssh_cmd = format!(
                        "while true; do ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=3 {ssh_target}; RC=$?; if [ $RC -eq 0 ]; then break; fi; echo '[연결 끊김 - 5초 후 재접속]'; sleep 5; done"
                    );
                    tmux::run(&["new-session", "-d", "-s", &session_name, &ssh_cmd])?;
                    tmux::switch_client(&format!("={session_name}"))?;
                }
                let _ = std::process::Command::new("tmux-sessionbar")
                    .args(["render-status", "left"])
                    .status();
            }
        }
    } else if let Some(user) = range.strip_prefix("_u") {
        tmux::run(&["set", "-g", "@view_user", user])?;

        let has = tmux::run(&["has-session", "-t", &format!("={user}")]).is_ok();

        if has {
            tmux::switch_client(&format!("={user}"))?;
        } else {
            tmux::run(&["new-session", "-d", "-s", user, &format!("sudo -iu {user}")])?;
            tmux::switch_client(&format!("={user}"))?;
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
            tmux::switch_client(&format!("={sess}:{win}"))?;
        }
    } else if let Some(target) = range.strip_prefix("_wx") {
        if let Some((sess, win)) = target.split_once('.') {
            let kill_cmd = format!("kill-window -t ={sess}:{win}");
            tmux::confirm(&format!("Kill window '{sess}:{win}'?"), &kill_cmd)?;
        }
    } else if let Some(target) = range.strip_prefix("_px") {
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            let kill_cmd = format!("kill-pane -t ={sess}:{win}.{pane}");
            tmux::confirm(&format!("Kill pane '{sess}.{win}.{pane}'?"), &kill_cmd)?;
        }
    } else if let Some(target) = range.strip_prefix("_pp") {
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            tmux::switch_client(&format!("={sess}:{win}"))?;
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
        tmux::command_prompt(&format!(
            "command-prompt -I \"{range}\" -p \"rename session:\" \"rename-session '%%'\""
        ))?;
    } else if let Some(idx) = range.strip_prefix("_ws") {
        tmux::command_prompt(&format!(
            "command-prompt -p \"rename window {idx}:\" \"rename-window -t :{idx} '%%'\""
        ))?;
    } else if let Some(target) = range.strip_prefix("_wa")
        && let Some((sess, win)) = target.split_once('.')
    {
        tmux::command_prompt(&format!(
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
    tmux::confirm(&format!("Kill window '{idx}'?"), &kill_cmd)?;

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
            tmux::switch_client(&format!("={}:{}", parts[0], parts[1]))?;
            return Ok(true);
        }
    }
    Ok(false)
}

