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

/// Get @view_user filter (empty = show all)
fn get_view_user() -> String {
    Command::new("tmux")
        .args(["show", "-gv", "@view_user"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Renders all windows across all sessions in session.window format
pub fn render_all_windows() -> Result<String, Box<dyn std::error::Error>> {
    let config = load_config()?;
    let w = &config.window;
    let view_user = get_view_user();

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
        let mut split = line.splitn(3, ':');
        let sess = split.next().unwrap_or("");

        // Filter by user if set
        if !view_user.is_empty() {
            let is_user_session = sess == view_user;
            let is_unowned = !sess.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false);
            let belongs_to_root = is_unowned && view_user == "root";
            if !is_user_session && !belongs_to_root {
                continue;
            }
        }
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


/// Renders panes for status-format
pub fn render_panes() -> Result<String, Box<dyn std::error::Error>> {
    let config = load_config()?;
    let w = &config.window;
    let view_user = get_view_user();

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

        // Filter by user if set
        if !view_user.is_empty() {
            let is_user_session = sess == view_user;
            let is_unowned = !sess.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false);
            let belongs_to_root = is_unowned && view_user == "root";
            if !is_user_session && !belongs_to_root {
                continue;
            }
        }

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

/// Get current view mode from tmux variable @view_mode
fn get_view_mode() -> String {
    Command::new("tmux")
        .args(["show", "-gv", "@view_mode"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "all".into())
}

/// Returns view switcher string for embedding in other lines
pub fn render_view_switcher() -> String {
    let mode = get_view_mode();

    let modes = [
        ("_vAll", "🌐", "#98c379"),
        ("_vUser", "👤", "#61afef"),
        ("_vSession", "📋", "#c678dd"),
        ("_vCompact", "⚡", "#e5c07b"),
    ];

    let mut parts = Vec::new();
    for (id, emoji, color) in &modes {
        let mode_name = id.strip_prefix("_v").unwrap_or(id).to_lowercase();
        if mode == mode_name {
            parts.push(format!(
                "#[range=user|{id}]#[fg=#282c34,bg={color},bold] {emoji} #[norange default]"
            ));
        } else {
            parts.push(format!(
                "#[range=user|{id}]#[fg=#abb2bf,bg=#3e4452] {emoji} #[norange default]"
            ));
        }
    }

    parts.join("")
}


pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let windows = render_windows()?;
    print!("{windows}");

    // Always 5 lines, always same structure
    // Line 0: Users
    render_line_users_at(0)?;
    // Line 1: Sessions (set by sessionbar)
    // Line 2: Windows (filtered by @view_user)
    render_line_windows_at(2)?;
    // Line 3: Panes (filtered by @view_user)
    render_line_panes_at(3)?;
    // Line 4: Apps
    render_line_apps_at(4)?;

    Command::new("tmux")
        .args(["set", "-g", "status", "5"])
        .status()?;

    Ok(())
}

// Helpers that render to a specific status-format index
fn render_line_users_at(idx: usize) -> Result<(), Box<dyn std::error::Error>> {
    render_line_users_impl(idx)
}

fn render_line_windows_at(idx: usize) -> Result<(), Box<dyn std::error::Error>> {
    let all_windows = render_all_windows()?;
    let label = "#[fg=#c678dd,bold]Windows #[default]";
    let format = format!("#[align=left default]{label}{all_windows}");
    Command::new("tmux")
        .args(["set", "-g", &format!("status-format[{idx}]"), &format])
        .status()?;
    Ok(())
}

fn render_line_panes_at(idx: usize) -> Result<(), Box<dyn std::error::Error>> {
    let panes = render_panes()?;
    let label = "#[fg=#e5c07b,bold]Panes #[default]";
    let format = format!("#[align=left default]{label}{panes}");
    Command::new("tmux")
        .args(["set", "-g", &format!("status-format[{idx}]"), &format])
        .status()?;
    Ok(())
}

fn render_line_apps_at(idx: usize) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    if config.apps.is_empty() {
        return Ok(());
    }
    let mut parts = Vec::new();
    for (i, app) in config.apps.iter().enumerate() {
        let range_id = format!("_app{i}");
        parts.push(format!(
            "#[range=user|{range_id}]#[fg={},bg={}] {} {} #[norange default]",
            app.fg, app.bg, app.emoji, app.command,
        ));
    }
    let label = "#[fg=#e06c75,bold]Apps #[default]";
    let format = format!("#[align=left default]{label}{}", parts.join(" "));
    Command::new("tmux")
        .args(["set", "-g", &format!("status-format[{idx}]"), &format])
        .status()?;
    Ok(())
}

fn render_line_users_impl(idx: usize) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let w = &config.window;
    let view_user = get_view_user();

    let current_user = std::env::var("USER").unwrap_or_else(|_| "root".into());

    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let mut users: Vec<&str> = Vec::new();
    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 7 { continue; }
        let name = fields[0];
        let uid: u32 = fields[2].parse().unwrap_or(0);
        let shell = fields[6];
        if shell.contains("nologin") || shell.contains("/false") { continue; }
        if uid == 0 || uid >= 1000 { users.push(name); }
    }

    let sessions_output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let active_sessions: Vec<&str> = sessions_output.lines().collect();

    let mut parts = Vec::new();
    for user in &users {
        let range_id = format!("_u{user}");
        if range_id.len() > 15 { continue; }
        let has_session = active_sessions.iter().any(|s| s == user);

        let is_viewed = !view_user.is_empty() && *user == view_user;

        let block = if is_viewed {
            // Currently filtered/viewed user
            format!(
                "#[range=user|{range_id}]#[fg=#282c34,bg=#e5c07b,bold] 👤 {user} #[norange]#[default]",
            )
        } else if *user == current_user {
            format!(
                "#[range=user|{range_id}]#[fg={},bg={},bold] 👤 {user} #[norange]#[default]",
                w.active_fg, w.active_bg,
            )
        } else if has_session {
            format!(
                "#[range=user|{range_id}]#[fg=#282c34,bg=#56b6c2] 👤 {user} #[norange]#[default]",
            )
        } else {
            format!(
                "#[range=user|{range_id}]#[fg={},bg={}] 👤 {user} #[norange]#[default]",
                w.fg, w.bg,
            )
        };
        parts.push(block);
    }

    let label = "#[fg=#56b6c2,bold]Users #[default]";
    let format = format!("#[align=left default]{label}{}", parts.join(" "));
    Command::new("tmux")
        .args(["set", "-g", &format!("status-format[{idx}]"), &format])
        .status()?;
    Ok(())
}
