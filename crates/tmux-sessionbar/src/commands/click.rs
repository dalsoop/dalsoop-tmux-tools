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

    // Use run-shell to execute switch+kill as a shell command,
    // so confirm-before can handle the full sequence
    let kill_cmd = if current == sess {
        format!(
            "run-shell 'tmux switch-client -l 2>/dev/null || tmux switch-client -n; tmux kill-session -t ={safe}'"
        )
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
