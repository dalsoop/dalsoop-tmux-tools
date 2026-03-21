use std::process::Command;

const CONFIRM_FILE: &str = "/tmp/tmux-pending-confirm.conf";

pub fn run(range: &str) -> Result<(), Box<dyn std::error::Error>> {
    if range == "window" {
        Command::new("tmux")
            .args(["select-window"])
            .status()?;
    } else if range == "_clear_" {
        Command::new("tmux")
            .args(["send-keys", "-R", ";", "clear-history"])
            .status()?;
    } else if range == "_new_" {
        Command::new("tmux")
            .args(["new-session", "-d"])
            .status()?;
    } else if let Some(sess) = range.strip_prefix("_k") {
        kill_session(sess)?;
    } else {
        Command::new("tmux")
            .args(["switch-client", "-t", &format!("={range}")])
            .status()?;
    }

    Ok(())
}

fn kill_session(sess: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()?;
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    let sessions: Vec<&str> = output_str
        .lines()
        .filter(|l| !l.is_empty())
        .collect();

    if sessions.len() <= 1 {
        Command::new("tmux")
            .args(["display-message", "cannot kill last session"])
            .status()?;
        return Ok(());
    }

    let current = Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let kill_cmd = if current == sess {
        format!("switch-client -n ; kill-session -t ={sess}")
    } else {
        format!("kill-session -t ={sess}")
    };

    // Write confirm to file — binding will source it after run-shell exits
    let content = format!(
        "confirm-before -p \"kill session '{sess}'? (y/n)\" \"{kill_cmd}\""
    );
    std::fs::write(CONFIRM_FILE, content)?;

    Ok(())
}
