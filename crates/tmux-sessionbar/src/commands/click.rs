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
        // Switch away before killing — try last session, then next
        if tmux::run(&["switch-client", "-l"]).is_err() {
            tmux::run(&["switch-client", "-n"])?;
        }
    }

    tmux::run(&["kill-session", "-t", &format!("={safe}")])?;

    Ok(())
}
