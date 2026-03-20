use crate::config::template::load_config;
use std::process::Command;

/// Renders current session's window list for status-format[0]
pub fn render_windows() -> Result<String, Box<dyn std::error::Error>> {
    let config = load_config()?;
    let w = &config.window;

    let current = Command::new("tmux")
        .args(["display-message", "-p", "#{window_index}"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let output = Command::new("tmux")
        .args(["list-windows", "-F", "#{window_index}:#{window_name}"])
        .output()?;
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();

    let mut parts = Vec::new();
    for line in output_str.lines() {
        if line.is_empty() {
            continue;
        }
        let (idx, name) = line.split_once(':').unwrap_or((line, ""));

        let block = if idx == current {
            format!(
                "#[range=user|_ws{idx}]#[fg={},bg={},bold] {idx}:{name} #[norange]#[default]",
                w.active_fg, w.active_bg,
            )
        } else {
            let mut b = format!(
                "#[range=user|_ws{idx}]#[fg={},bg={}] {idx}:{name} #[norange]",
                w.fg, w.bg,
            );
            if w.show_kill_button {
                b.push_str(&format!(
                    "#[range=user|_wk{idx}]#[fg={},bg={}] x #[norange default]",
                    w.kill_fg, w.kill_bg,
                ));
            } else {
                b.push_str("#[default]");
            }
            b
        };

        parts.push(block);
    }

    let mut result = parts.join(" ");

    if w.show_new_button {
        result.push_str(&format!(
            " #[range=user|_wnew_]#[fg={},bg={}] + #[norange default]",
            w.button_fg, w.button_bg,
        ));
    }

    Ok(result)
}

/// Renders all windows across all sessions in session.window format for status-format[1]
pub fn render_all_windows() -> Result<String, Box<dyn std::error::Error>> {
    let config = load_config()?;
    let w = &config.window;

    // Get current session and window
    let current_session = Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let current_window = Command::new("tmux")
        .args(["display-message", "-p", "#{window_index}"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // List all windows across all sessions
    let output = Command::new("tmux")
        .args([
            "list-windows",
            "-a",
            "-F",
            "#{session_name}:#{window_index}:#{window_name}",
        ])
        .output()?;
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();

    let mut parts = Vec::new();
    for line in output_str.lines() {
        if line.is_empty() {
            continue;
        }
        // Parse session:index:name
        let mut split = line.splitn(3, ':');
        let sess = split.next().unwrap_or("");
        let idx = split.next().unwrap_or("");
        let name = split.next().unwrap_or("");

        let is_active = sess == current_session && idx == current_window;
        // range id: _wa{session}.{index} (all-windows switch)
        // Keep under 15 bytes
        let range_id = format!("_wa{sess}.{idx}");

        let block = if is_active {
            format!(
                "#[range=user|{range_id}]#[fg={},bg={},bold] {sess}.{idx}:{name} #[norange]#[default]",
                w.active_fg, w.active_bg,
            )
        } else {
            let kill_id = format!("_wx{sess}.{idx}");
            format!(
                "#[range=user|{range_id}]#[fg={},bg={}] {sess}.{idx}:{name} #[norange]\
                 #[range=user|{kill_id}]#[fg={},bg={}] x #[norange default]",
                w.fg, w.bg, w.kill_fg, w.kill_bg,
            )
        };

        parts.push(block);
    }

    Ok(parts.join(" "))
}

/// Set status-format[1] with all windows
pub fn render_line2() -> Result<(), Box<dyn std::error::Error>> {
    let all_windows = render_all_windows()?;

    let label = "#[fg=#c678dd,bold]Windows #[default]";
    let format = format!("#[align=left default]{label}{all_windows}");

    Command::new("tmux")
        .args(["set", "-g", "status-format[1]", &format])
        .status()?;

    // Ensure 2-line status
    Command::new("tmux")
        .args(["set", "-g", "status", "2"])
        .status()?;

    Ok(())
}

/// Renders panes of the current window for status-format[2]
pub fn render_panes() -> Result<String, Box<dyn std::error::Error>> {
    let config = load_config()?;
    let w = &config.window;

    let current_session = Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let current_window = Command::new("tmux")
        .args(["display-message", "-p", "#{window_index}"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let current_pane = Command::new("tmux")
        .args(["display-message", "-p", "#{pane_index}"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // List all panes across all sessions
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-a",
            "-F",
            "#{session_name}:#{window_index}:#{pane_index}:#{pane_current_command}",
        ])
        .output()?;
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();

    let mut parts = Vec::new();
    for line in output_str.lines() {
        if line.is_empty() {
            continue;
        }
        let mut split = line.splitn(4, ':');
        let sess = split.next().unwrap_or("");
        let win = split.next().unwrap_or("");
        let pane = split.next().unwrap_or("");
        let cmd = split.next().unwrap_or("");

        let is_active = sess == current_session && win == current_window && pane == current_pane;
        // range id: _pp{s}.{w}.{p} — keep under 15 bytes
        let range_id = format!("_pp{sess}.{win}.{pane}");

        let is_idle = matches!(cmd, "bash" | "zsh" | "fish" | "sh" | "dash" | "ksh" | "csh" | "tcsh");

        let block = if is_active {
            format!(
                "#[range=user|{range_id}]#[fg={},bg={},bold] {sess}.{win}.{pane}:{cmd} #[norange]#[default]",
                w.active_fg, w.active_bg,
            )
        } else {
            let (fg, bg) = if let Some(c) = config.colors.get(cmd) {
                (c.fg.clone(), c.bg.clone())
            } else if is_idle {
                (w.idle_fg.clone(), w.idle_bg.clone())
            } else {
                (w.running_fg.clone(), w.running_bg.clone())
            };
            let kill_id = format!("_px{sess}.{win}.{pane}");
            format!(
                "#[range=user|{range_id}]#[fg={fg},bg={bg}] {sess}.{win}.{pane}:{cmd} #[norange]\
                 #[range=user|{kill_id}]#[fg={},bg={}] x #[norange default]",
                w.kill_fg, w.kill_bg,
            )
        };

        parts.push(block);
    }

    let mut result = parts.join(" ");

    // Split buttons
    result.push_str(&format!(
        " #[range=user|_splith]#[fg={},bg={}] | #[norange default]\
         #[range=user|_splitv]#[fg={},bg={}] - #[norange default]",
        w.button_fg, w.button_bg, w.button_fg, w.button_bg,
    ));

    Ok(result)
}

/// Set status-format[2] with panes
pub fn render_line3() -> Result<(), Box<dyn std::error::Error>> {
    let panes = render_panes()?;

    let label = "#[fg=#e5c07b,bold]Panes #[default]";
    let format = format!("#[align=left default]{label}{panes}");

    Command::new("tmux")
        .args(["set", "-g", "status-format[2]", &format])
        .status()?;

    // Ensure 3-line status
    Command::new("tmux")
        .args(["set", "-g", "status", "3"])
        .status()?;

    Ok(())
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let windows = render_windows()?;
    print!("{windows}");

    // Update line 2 (all windows) and line 3 (panes)
    render_line2()?;
    render_line3()?;

    Ok(())
}
