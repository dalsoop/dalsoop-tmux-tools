use crate::config::template::load_config;
use std::process::Command;

/// Renders window list and returns the formatted string for status-format[0]
pub fn render_windows() -> Result<String, Box<dyn std::error::Error>> {
    let config = load_config()?;
    let w = &config.window;

    // Get current window index
    let current = Command::new("tmux")
        .args(["display-message", "-p", "#{window_index}"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // List all windows: index:name
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

        let mut block = if idx == current {
            // Active window: no [x]
            format!(
                "#[range=user|_ws{idx}]#[fg={},bg={},bold] {idx}:{name} #[norange]#[default]",
                w.active_fg, w.active_bg,
            )
        } else {
            // Inactive window: with [x]
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

    // [+] new window button
    if w.show_new_button {
        result.push_str(&format!(
            " #[range=user|_wnew_]#[fg={},bg={}] + #[norange default]",
            w.button_fg, w.button_bg,
        ));
    }

    Ok(result)
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let windows = render_windows()?;
    print!("{windows}");
    Ok(())
}
