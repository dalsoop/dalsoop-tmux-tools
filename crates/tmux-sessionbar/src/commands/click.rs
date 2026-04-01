use anyhow::Result;
use tmux_fmt::tmux;
use tmux_fmt::tmux::sanitize as sanitize_tmux;

pub fn run(range: &str) -> Result<()> {
    if range == "window" {
        tmux::run(&["select-window"])?;
    } else if range == "_new_" {
        tmux::run(&["new-session", "-d"])?;
    } else if let Some(idx_str) = range.strip_prefix("_k") {
        // Kill by index — resolve to session name
        if let Some(name) = resolve_session_by_index(idx_str) {
            kill_session(&name)?;
        }
    } else if let Some(idx_str) = range.strip_prefix("_s") {
        // Switch by index — resolve to session name
        if let Some(name) = resolve_session_by_index(idx_str) {
            tmux::switch_client(&format!("={name}"))?;
        }
    } else {
        // Fallback: treat range as session name (for backward compat)
        tmux::switch_client(&format!("={range}"))?;
    }

    Ok(())
}

/// Resolve a visible session index to its name.
///
/// The index corresponds to the order sessions appear in the status bar
/// (filtered by @view_user).
fn resolve_session_by_index(idx_str: &str) -> Option<String> {
    let idx: usize = idx_str.parse().ok()?;
    let sessions = tmux::lines(&["list-sessions", "-F", "#{session_name}"]).ok()?;
    let view_user = tmux::query_or(&["show", "-gv", "@view_user"], "");
    let visible: Vec<&String> = sessions
        .iter()
        .filter(|n| tmux::should_show_for_user(n, &view_user))
        .collect();
    visible.get(idx).map(|s| s.to_string())
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
        let switch_cmd = if let Ok(client) = std::env::var("TMUX_CLIENT") {
            let client = tmux::sanitize(&client);
            format!("run-shell 'tmux switch-client -c {client} -l 2>/dev/null || tmux switch-client -c {client} -n; tmux kill-session -t ={safe}'")
        } else {
            format!("run-shell 'tmux switch-client -l 2>/dev/null || tmux switch-client -n; tmux kill-session -t ={safe}'")
        };
        tmux::confirm_raw(&format!("kill session '{safe}'?"), &switch_cmd)?;
    } else {
        tmux::confirm(&format!("kill session '{safe}'?"), &format!("kill-session -t ={safe}"))?;
    }

    Ok(())
}
