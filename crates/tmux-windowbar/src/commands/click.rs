use crate::config::template::load_config;
use std::process::Command;

pub fn run(range: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mode) = range.strip_prefix("_v") {
        // View mode switch
        let mode = mode.to_lowercase();
        Command::new("tmux")
            .args(["set", "-g", "@view_mode", &mode])
            .status()?;
        if mode == "all" {
            // Clear user filter
            let _ = Command::new("tmux")
                .args(["set", "-gu", "@view_user"])
                .status();
        }
        // Re-render
        let _ = Command::new("tmux-sessionbar")
            .args(["render-status", "left"])
            .status();
    } else if let Some(idx_str) = range.strip_prefix("_app") {
        if let Ok(idx) = idx_str.parse::<usize>() {
            let config = load_config()?;
            if let Some(app) = config.apps.get(idx) {
                if app.mode == "pane" {
                    Command::new("tmux")
                        .args(["split-window", "-h", &app.command])
                        .status()?;
                } else {
                    Command::new("tmux")
                        .args(["new-window", "-n", &app.command, &app.command])
                        .status()?;
                }
            }
        }
    } else if let Some(user) = range.strip_prefix("_u") {
        // Set view filter to this user
        Command::new("tmux")
            .args(["set", "-g", "@view_user", user])
            .status()?;

        // Check if session for this user already exists
        let check = Command::new("tmux")
            .args(["has-session", "-t", &format!("={user}")])
            .status();
        if check.map(|s| s.success()).unwrap_or(false) {
            Command::new("tmux")
                .args(["switch-client", "-t", &format!("={user}")])
                .status()?;
        } else {
            Command::new("tmux")
                .args(["new-session", "-d", "-s", user, &format!("sudo -iu {user}")])
                .status()?;
            Command::new("tmux")
                .args(["switch-client", "-t", &format!("={user}")])
                .status()?;
        }
        // Force re-render with new filter
        let _ = Command::new("tmux-sessionbar")
            .args(["render-status", "left"])
            .status();
    } else if range == "_splith" {
        // Horizontal split (side by side)
        Command::new("tmux")
            .args(["split-window", "-h"])
            .status()?;
    } else if range == "_splitv" {
        // Vertical split (top/bottom)
        Command::new("tmux")
            .args(["split-window", "-v"])
            .status()?;
    } else if range == "_wnew_" {
        Command::new("tmux")
            .args(["new-window"])
            .status()?;
    } else if let Some(idx) = range.strip_prefix("_wk") {
        kill_window(idx)?;
    } else if let Some(idx) = range.strip_prefix("_ws") {
        // Window switch (current session)
        Command::new("tmux")
            .args(["select-window", "-t", &format!(":{idx}")])
            .status()?;
    } else if let Some(target) = range.strip_prefix("_wa") {
        // All-windows switch: target is "session.window"
        if let Some((sess, win)) = target.split_once('.') {
            Command::new("tmux")
                .args(["switch-client", "-t", &format!("={sess}:{win}")])
                .status()?;
        }
    } else if let Some(target) = range.strip_prefix("_wx") {
        // Kill window: target is "session.window"
        if let Some((sess, win)) = target.split_once('.') {
            let kill_cmd = format!("kill-window -t ={sess}:{win}");
            confirm_and_run(&format!("Kill window '{sess}:{win}'?"), &kill_cmd)?;
        }
    } else if let Some(target) = range.strip_prefix("_px") {
        // Kill pane: target is "session.window.pane"
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            let kill_cmd = format!("kill-pane -t ={sess}:{win}.{pane}");
            confirm_and_run(&format!("Kill pane '{sess}.{win}.{pane}'?"), &kill_cmd)?;
        }
    } else if let Some(target) = range.strip_prefix("_pp") {
        // Pane select: target is "session.window.pane"
        let parts: Vec<&str> = target.splitn(3, '.').collect();
        if parts.len() == 3 {
            let (sess, win, pane) = (parts[0], parts[1], parts[2]);
            // Switch to session+window first, then select pane
            Command::new("tmux")
                .args(["switch-client", "-t", &format!("={sess}:{win}")])
                .status()?;
            Command::new("tmux")
                .args(["select-pane", "-t", &format!("={sess}:{win}.{pane}")])
                .status()?;
        }
    } else {
        // Not our range, exit with error so sessionbar can handle it
        return Err(format!("unknown range: {range}").into());
    }

    Ok(())
}

fn kill_window(idx: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Count windows
    let output = Command::new("tmux")
        .args(["list-windows", "-F", "#{window_index}"])
        .output()?;
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    let windows: Vec<&str> = output_str
        .lines()
        .filter(|l| !l.is_empty())
        .collect();

    if windows.len() <= 1 {
        Command::new("tmux")
            .args(["display-message", "cannot kill last window"])
            .status()?;
        return Ok(());
    }

    let kill_cmd = format!("kill-window -t :{idx}");
    confirm_and_run(&format!("Kill window '{idx}'?"), &kill_cmd)?;

    Ok(())
}

/// Write confirm-before to pending file for binding to pick up
fn confirm_and_run(title: &str, cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = format!(
        "confirm-before -p \"{title} (y/n)\" \"{cmd}\""
    );
    std::fs::write("/tmp/tmux-pending-confirm.conf", content)?;
    Ok(())
}
