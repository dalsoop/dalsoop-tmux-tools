use anyhow::Result;
use tmux_fmt::tmux;
use tmux_fmt::tmux::sanitize as sanitize_tmux;

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

    if current == sess {
        // Active session: switch away first via run-shell, so confirm-before
        // can handle the full sequence as a single command
        let cmd = format!(
            "run-shell 'tmux switch-client -l 2>/dev/null || tmux switch-client -n; tmux kill-session -t ={safe}'"
        );
        tmux::confirm_raw(&format!("kill session '{safe}'?"), &cmd)?;
    } else {
        tmux::confirm(&format!("kill session '{safe}'?"), &format!("kill-session -t ={safe}"))?;
    }

    Ok(())
}
