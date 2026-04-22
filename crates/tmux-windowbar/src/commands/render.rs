use crate::config::template::{Config, SshEntry, load_config};
use anyhow::Result;
use tmux_fmt::tmux;
use tmux_fmt::{Line, click, label, styled};

/// Renders current session's window list for status-format[0]
fn render_windows(config: &Config) -> Result<String> {
    let w = &config.window;

    let current = tmux::query_or(&["display-message", "-p", "#{window_index}"], "");

    let lines = tmux::lines(&["list-windows", "-F", "#{window_index}:#{window_name}"])?;

    let mut parts = Vec::new();
    for line in &lines {
        let (idx, name) = line.split_once(':').unwrap_or((line, ""));

        let block = if idx == current {
            click(
                &format!("_ws{idx}"),
                &w.active_fg,
                &w.active_bg,
                true,
                &format!(" {idx}:{name} "),
            )
        } else {
            let mut b = click(
                &format!("_ws{idx}"),
                &w.fg,
                &w.bg,
                false,
                &format!(" {idx}:{name} "),
            );
            if w.show_kill_button {
                b.push_str(&click(
                    &format!("_wk{idx}"),
                    &w.kill_fg,
                    &w.kill_bg,
                    false,
                    " x ",
                ));
            }
            b
        };

        parts.push(block);
    }

    let mut result = parts.join(" ");

    if w.show_new_button {
        result.push_str(&format!(
            " {}",
            click("_wnew_", &w.button_fg, &w.button_bg, false, " + ")
        ));
    }

    Ok(result)
}

/// Get @view_user filter (empty = show all)
fn get_view_user() -> String {
    tmux::query_or(&["show", "-gv", "@view_user"], "")
}

/// Renders all windows across all sessions in session.window format
fn render_all_windows(config: &Config) -> Result<String> {
    let w = &config.window;
    let view_user = get_view_user();

    let current_session = tmux::query_or(&["display-message", "-p", "#S"], "");
    let current_window = tmux::query_or(&["display-message", "-p", "#{window_index}"], "");

    let lines = tmux::lines(&[
        "list-windows",
        "-a",
        "-F",
        "#{session_name}:#{window_index}:#{window_name}",
    ])?;

    let mut parts = Vec::new();
    for line in &lines {
        let mut split = line.splitn(3, ':');
        let sess = split.next().unwrap_or("");

        if !tmux::should_show_for_user(sess, &view_user) {
            continue;
        }
        let idx = split.next().unwrap_or("");
        let name = split.next().unwrap_or("");

        let is_active = sess == current_session && idx == current_window;
        let range_id = format!("_wa{sess}.{idx}");

        let display = format!(" {sess}.{idx}:{name} ");
        let kill_id = format!("_wx{sess}.{idx}");
        let (fg, bg, bold) = if is_active {
            (&w.active_fg, &w.active_bg, true)
        } else {
            (&w.fg, &w.bg, false)
        };
        let mut block = click(&range_id, fg, bg, bold, &display);
        block.push_str(&click(&kill_id, &w.kill_fg, &w.kill_bg, false, " x "));

        parts.push(block);
    }

    let mut result = parts.join(" ");
    if w.show_new_button {
        result.push_str(&format!(
            " {}",
            click("_wnew_", &w.button_fg, &w.button_bg, false, " + ")
        ));
    }

    Ok(result)
}

/// Renders panes for status-format
fn render_panes(config: &Config) -> Result<String> {
    let w = &config.window;
    let view_user = get_view_user();

    let current_session = tmux::query_or(&["display-message", "-p", "#S"], "");
    let current_window = tmux::query_or(&["display-message", "-p", "#{window_index}"], "");
    let current_pane = tmux::query_or(&["display-message", "-p", "#{pane_index}"], "");

    let lines = tmux::lines(&[
        "list-panes",
        "-a",
        "-F",
        "#{session_name}:#{window_index}:#{pane_index}:#{pane_current_command}",
    ])?;

    let mut parts = Vec::new();
    for line in &lines {
        let mut split = line.splitn(4, ':');
        let sess = split.next().unwrap_or("");
        let win = split.next().unwrap_or("");
        let pane = split.next().unwrap_or("");
        let cmd = split.next().unwrap_or("");

        if !tmux::should_show_for_user(sess, &view_user) {
            continue;
        }

        let is_active = sess == current_session && win == current_window && pane == current_pane;
        let range_id = format!("_pp{sess}.{win}.{pane}");
        let display = format!(" {sess}.{win}.{pane}:{cmd} ");

        let is_idle = matches!(
            cmd,
            "bash" | "zsh" | "fish" | "sh" | "dash" | "ksh" | "csh" | "tcsh"
        );

        let kill_id = format!("_px{sess}.{win}.{pane}");
        let block = if is_active {
            let mut b = click(&range_id, &w.active_fg, &w.active_bg, true, &display);
            b.push_str(&click(&kill_id, &w.kill_fg, &w.kill_bg, false, " x "));
            b
        } else {
            let (fg, bg) = if let Some(c) = config.colors.get(cmd) {
                (c.fg.clone(), c.bg.clone())
            } else if is_idle {
                (w.idle_fg.clone(), w.idle_bg.clone())
            } else {
                (w.running_fg.clone(), w.running_bg.clone())
            };
            let mut b = click(&range_id, &fg, &bg, false, &display);
            b.push_str(&click(&kill_id, &w.kill_fg, &w.kill_bg, false, " x "));
            b
        };

        parts.push(block);
    }

    let mut result = parts.join(" ");

    // Split buttons
    result.push_str(&format!(
        " {}{}",
        click("_splith", &w.button_fg, &w.button_bg, false, " | "),
        click("_splitv", &w.button_fg, &w.button_bg, false, " - "),
    ));

    Ok(result)
}

pub fn run() -> Result<()> {
    if !tmux::acquire_guard("windowbar_render", 100) {
        return Ok(());
    }

    let config = load_config()?;

    let windows = render_windows(&config)?;
    print!("{windows}");

    render_line_users(&config, 0)?;
    render_line_windows(&config, 2)?;
    render_line_panes(&config, 3)?;
    render_line_apps(&config, 4)?;

    Ok(())
}

fn render_line_users(config: &Config, idx: usize) -> Result<()> {
    let w = &config.window;
    let th = &config.theme;
    let view_user = get_view_user();

    let current_user = std::env::var("USER").unwrap_or_else(|_| "root".into()); // LINT_ALLOW: last-resort fallback when USER is unset

    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let mut users: Vec<&str> = Vec::new();
    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 7 {
            continue;
        }
        let name = fields[0];
        let uid: u32 = fields[2].parse().unwrap_or(0);
        let shell = fields[6];
        if shell.contains("nologin") || shell.contains("/false") {
            continue;
        }
        if uid == 0 || uid >= 1000 {
            users.push(name);
        }
    }

    let active_sessions =
        tmux::lines(&["list-sessions", "-F", "#{session_name}"]).unwrap_or_default();

    let mut parts = Vec::new();
    for user in &users {
        let range_id = format!("_u{user}");
        if range_id.len() > 15 {
            continue;
        }
        let has_session = active_sessions.iter().any(|s| s == user);

        let is_viewed = !view_user.is_empty() && *user == view_user;

        let block = if is_viewed {
            click(
                &range_id,
                &th.user_viewed_fg,
                &th.user_viewed_bg,
                true,
                &format!(" 👤 {user} "),
            )
        } else if *user == current_user {
            click(
                &range_id,
                &w.active_fg,
                &w.active_bg,
                true,
                &format!(" 👤 {user} "),
            )
        } else if has_session {
            click(
                &range_id,
                &th.user_session_fg,
                &th.user_session_bg,
                false,
                &format!(" 👤 {user} "),
            )
        } else {
            click(&range_id, &w.fg, &w.bg, false, &format!(" 👤 {user} "))
        };
        parts.push(block);
    }

    // SSH hosts on same line, after users
    let mut ssh_parts = Vec::new();
    let active_sessions =
        tmux::lines(&["list-sessions", "-F", "#{session_name}"]).unwrap_or_default();
    let reachability = check_ssh_reachability(&config.ssh);
    for (i, entry) in config.ssh.iter().enumerate() {
        let range_id = format!("_ssh{i}");
        let session_name = format!("ssh-{}", entry.name);
        let has_session = active_sessions.contains(&session_name);
        let is_reachable = reachability.get(i).copied().unwrap_or(true);

        let block = if has_session && !is_reachable {
            // Session exists but host unreachable — red warning
            click(
                &range_id,
                &th.ssh_unreachable_fg,
                &th.ssh_unreachable_bg,
                true,
                &format!(" {} {} ⚠ ", entry.emoji, entry.name),
            )
        } else if has_session {
            click(
                &range_id,
                &th.ssh_connected_fg,
                &th.ssh_connected_bg,
                true,
                &format!(" {} {} ", entry.emoji, entry.name),
            )
        } else if !is_reachable {
            // No session and unreachable — red dimmed
            click(
                &range_id,
                &th.ssh_unreachable_fg,
                &th.ssh_unreachable_bg,
                false,
                &format!(" {} {} ⚠ ", entry.emoji, entry.name),
            )
        } else {
            click(
                &range_id,
                &entry.fg,
                &entry.bg,
                false,
                &format!(" {} {} ", entry.emoji, entry.name),
            )
        };
        ssh_parts.push(block);
    }

    let btn_fg = &w.button_fg;
    let btn_bg = &w.button_bg;
    let pane_controls = format!(
        "{}{}{}",
        click("_nextlayout", btn_fg, btn_bg, false, "  ⊞  "),
        click("_zoom", btn_fg, btn_bg, false, "  ⤢  "),
        click("_rotate", btn_fg, btn_bg, false, "  ↻  "),
    );

    // VPN status indicator
    let vpn_badge = match get_vpn_status() {
        Some(vpn) => styled(
            &th.vpn_connected_fg,
            &th.vpn_connected_bg,
            &format!(" 🔒 {} ", vpn.ip),
        ),
        None => styled(
            &th.vpn_disconnected_fg,
            &th.vpn_disconnected_bg,
            " 🔓 VPN ✗ ",
        ),
    };

    let mut line = Line::new().left().push(&label("Users", &th.users_label));
    line = line.push(&parts.join(" "));
    if !ssh_parts.is_empty() {
        line = line.push("  ").push(&ssh_parts.join(" "));
    }
    let format = line.right().push(&vpn_badge).push(" ").push(&pane_controls).build();
    tmux::run(&["set", "-g", &format!("status-format[{idx}]"), &format])?;
    Ok(())
}

fn render_line_windows(config: &Config, idx: usize) -> Result<()> {
    let all_windows = render_all_windows(config)?;
    let format = Line::new()
        .left()
        .push(&label("Windows", &config.theme.windows_label))
        .push(&all_windows)
        .build();
    tmux::run(&["set", "-g", &format!("status-format[{idx}]"), &format])?;
    Ok(())
}

fn render_line_panes(config: &Config, idx: usize) -> Result<()> {
    let panes = render_panes(config)?;
    let format = Line::new()
        .left()
        .push(&label("Panes", &config.theme.panes_label))
        .push(&panes)
        .build();
    tmux::run(&["set", "-g", &format!("status-format[{idx}]"), &format])?;
    Ok(())
}

fn render_line_apps(config: &Config, idx: usize) -> Result<()> {
    let all: Vec<_> = config.all_apps().collect();
    if all.is_empty() {
        return Ok(());
    }
    let mut parts = Vec::new();
    for (i, app) in all.iter().enumerate() {
        let range_id = format!("_app{i}");
        parts.push(click(
            &range_id,
            &app.fg,
            &app.bg,
            false,
            &format!(" {} {} ", app.emoji, app.command),
        ));
    }
    let format = Line::new()
        .left()
        .push(&label("Apps", &config.theme.apps_label))
        .push(&parts.join(" "))
        .build();
    tmux::run(&["set", "-g", &format!("status-format[{idx}]"), &format])?;
    Ok(())
}

// ── Network reachability ──

/// Health check refresh interval in seconds.
const HEALTH_CHECK_INTERVAL: u64 = 15;

/// Check if a host IP has a specific (non-default) route.
/// On macOS, parses `route -n get <ip>` output.
fn has_route_to(ip: &str) -> bool {
    let output = std::process::Command::new("route")
        .args(["-n", "get", ip])
        .stderr(std::process::Stdio::null())
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Some(dest) = trimmed.strip_prefix("destination:") {
                    return dest.trim() != "default";
                }
            }
            true
        }
        _ => true,
    }
}

/// Check reachability for SSH entries using cached TCP probes.
///
/// Results are cached in tmux `@ssh_health` variable. A background
/// `nc -z -w 1` check runs every 15 seconds per unique /24 subnet.
/// Between refreshes, the cached result is returned instantly.
/// On the very first call (no cache), falls back to route-based check.
fn check_ssh_reachability(entries: &[SshEntry]) -> Vec<bool> {
    use std::collections::HashMap;

    if entries.is_empty() {
        return Vec::new();
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cache_ts: u64 = tmux::query_or(&["show", "-gv", "@ssh_health_ts"], "0")
        .parse()
        .unwrap_or(0);
    let cache_str = tmux::query_or(&["show", "-gv", "@ssh_health"], "");

    // Parse cache: "ip1=1,ip2=0,ip3=1,"
    let mut cached: HashMap<&str, bool> = HashMap::new();
    for part in cache_str.split(',') {
        if let Some((ip, val)) = part.split_once('=') {
            cached.insert(ip, val == "1");
        }
    }

    // Spawn background refresh if stale
    if now.saturating_sub(cache_ts) >= HEALTH_CHECK_INTERVAL {
        tmux::run_quiet(&["set", "-g", "@ssh_health_ts", &now.to_string()]);
        spawn_health_check(entries);
    }

    // Use cache if available, otherwise fall back to route check
    if !cached.is_empty() {
        entries
            .iter()
            .map(|e| cached.get(e.host.as_str()).copied().unwrap_or(true))
            .collect()
    } else {
        // First run: fast route-based approximation while TCP check runs
        let mut subnet_cache: HashMap<String, bool> = HashMap::new();
        entries
            .iter()
            .map(|entry| {
                let parts: Vec<&str> = entry.host.split('.').collect();
                if parts.len() != 4 {
                    return true;
                }
                let subnet_key = format!("{}.{}.{}.0", parts[0], parts[1], parts[2]);
                *subnet_cache
                    .entry(subnet_key)
                    .or_insert_with(|| has_route_to(&entry.host))
            })
            .collect()
    }
}

/// Spawn a background TCP health check via `tmux run-shell -b`.
/// Checks one representative host per /24 subnet using `nc -z -w 1`,
/// then updates `@ssh_health` with results.
fn spawn_health_check(entries: &[SshEntry]) {
    if entries.is_empty() {
        return;
    }

    // Group by /24 subnet
    let mut subnets: std::collections::HashMap<String, Vec<&str>> =
        std::collections::HashMap::new();
    for entry in entries {
        let parts: Vec<&str> = entry.host.split('.').collect();
        if parts.len() == 4 {
            let subnet = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
            subnets.entry(subnet).or_default().push(&entry.host);
        }
    }

    // Build shell script: check one host per subnet, apply result to all
    let mut script = String::from("r=; ");
    for hosts in subnets.values() {
        let rep = hosts[0];
        script.push_str(&format!(
            "if nc -z -w 1 {rep} 22 >/dev/null 2>&1; then v=1; else v=0; fi; "
        ));
        for host in hosts {
            script.push_str(&format!("r=\"${{r}}{host}=$v,\"; "));
        }
    }
    script.push_str("tmux set -g @ssh_health \"$r\"");

    tmux::run_quiet(&["run-shell", "-b", &script]);
}

// ── VPN status ──

struct VpnInfo {
    ip: String,
}

/// Detect active VPN connection via macOS `scutil --nc list`.
/// Returns VPN IP if any VPN profile is Connected.
fn get_vpn_status() -> Option<VpnInfo> {
    let output = std::process::Command::new("scutil")
        .args(["--nc", "list"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find a Connected VPN profile and extract UUID (3rd token)
    // Format: "* (Connected)  <UUID> VPN (type) ..."
    let connected = stdout.lines().find(|line| line.contains("(Connected)"))?;
    let uuid = connected.split_whitespace().nth(2)?;

    // Query status by UUID to get IP
    let status_output = std::process::Command::new("scutil")
        .args(["--nc", "status", uuid])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    let status_str = String::from_utf8_lossy(&status_output.stdout);

    // Parse "0 : <vpn-ip>" after "Addresses" section
    let mut in_addresses = false;
    for line in status_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Addresses") {
            in_addresses = true;
            continue;
        }
        if in_addresses {
            // Format: "0 : <ip-address>"
            if let Some(ip) = trimmed.split(" : ").nth(1) {
                return Some(VpnInfo {
                    ip: ip.trim().to_string(),
                });
            }
            in_addresses = false;
        }
    }

    // Fallback: connected but couldn't parse IP
    Some(VpnInfo {
        ip: "connected".to_string(),
    })
}
