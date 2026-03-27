use anyhow::Result;
use tmux_fmt::tmux;
use tmux_fmt::tmux::sanitize as sanitize_tmux;

const CONFIRM_FILE: &str = "/tmp/tmux-pending-confirm.conf";

pub fn run(range: &str) -> Result<()> {
    if range == "window" {
        tmux::run(&["select-window"])?;
    } else if range == "_clear_" {
        tmux::run(&["send-keys", "-R", ";", "clear-history"])?;
    } else if range == "_new_" {
        tmux::run(&["new-session", "-d"])?;
    } else if let Some(sess) = range.strip_prefix("_k") {
        kill_session(sess)?;
    } else {
        tmux::run(&["switch-client", "-t", &format!("={range}")])?;
    }

    apply_pending_confirm()?;

    Ok(())
}

fn kill_session(sess: &str) -> Result<()> {
    let sessions = tmux::lines(&["list-sessions", "-F", "#{session_name}"])?;

    if sessions.len() <= 1 {
        tmux::run(&["display-message", "cannot kill last session"])?;
        return Ok(());
    }

    let current = tmux::query_or(&["display-message", "-p", "#S"], "");

    let safe = sanitize_tmux(sess);
    let kill_cmd = if current == sess {
        format!("switch-client -n ; kill-session -t ={safe}")
    } else {
        format!("kill-session -t ={safe}")
    };

    let content = format!(
        "confirm-before -p \"kill session '{}'? (y/n)\" \"{}\"",
        safe, kill_cmd,
    );
    std::fs::write(CONFIRM_FILE, content)?;

    Ok(())
}

fn apply_pending_confirm() -> Result<()> {
    if !std::path::Path::new(CONFIRM_FILE).exists() {
        return Ok(());
    }

    tmux::run(&["source-file", CONFIRM_FILE])?;
    let _ = std::fs::remove_file(CONFIRM_FILE);
    Ok(())
}
