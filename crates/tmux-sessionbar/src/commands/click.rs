use std::process::Command;

pub fn run(range: &str) -> Result<(), Box<dyn std::error::Error>> {
    if range == "window" {
        Command::new("tmux")
            .args(["select-window"])
            .status()?;
    } else if range == "_new_" {
        Command::new("tmux")
            .args(["new-session", "-d"])
            .status()?;
    } else if let Some(sess) = range.strip_prefix("_kill_") {
        kill_session(sess)?;
    } else {
        Command::new("tmux")
            .args(["switch-client", "-t", &format!("={range}")])
            .status()?;
    }

    Ok(())
}

fn kill_session(sess: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Count sessions
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()?;
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    let sessions: Vec<&str> = output_str
        .lines()
        .filter(|l| !l.is_empty())
        .collect();

    // Don't kill the last session
    if sessions.len() <= 1 {
        Command::new("tmux")
            .args(["display-message", "cannot kill last session"])
            .status()?;
        return Ok(());
    }

    // If killing the current session, switch to another first
    let current = Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // Build the kill command — if it's the current session, switch first
    let kill_cmd = if current == sess {
        // Find another session to switch to
        format!("switch-client -n \\; kill-session -t ={sess}")
    } else {
        format!("kill-session -t ={sess}")
    };

    Command::new("tmux")
        .args([
            "confirm-before",
            "-p",
            &format!("kill session '{sess}'? (y/n)"),
            &kill_cmd,
        ])
        .status()?;

    Ok(())
}
