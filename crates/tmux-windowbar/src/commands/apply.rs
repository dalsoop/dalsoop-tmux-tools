use crate::config::template;
use std::process::Command;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = template::config_path();

    if !config_path.exists() {
        return Err(format!(
            "config not found: {}\nrun `tmux-windowbar init` first.",
            config_path.display()
        )
        .into());
    }

    apply_settings()?;
    println!("tmux-windowbar applied");
    Ok(())
}

pub fn apply_settings() -> Result<(), Box<dyn std::error::Error>> {
    let binary_path = std::env::current_exe()?
        .to_string_lossy()
        .to_string();

    // Mouse binding: chain windowbar click -> sessionbar click
    let binding = format!(
        "if-shell -F '1' \"run-shell '{binary_path} click \\\"#{{mouse_status_range}}\\\" 2>/dev/null || tmux-sessionbar click \\\"#{{mouse_status_range}}\\\" 2>/dev/null'\""
    );
    Command::new("tmux")
        .args(["bind", "-Troot", "MouseDown1Status", &binding])
        .status()?;

    // Trigger sessionbar re-render which will call windowbar render
    let _ = Command::new("tmux-sessionbar")
        .args(["render-status", "left"])
        .status();

    // Add hooks for window events
    let hook_cmd = format!("run-shell -b 'tmux-sessionbar render-status left'");
    Command::new("tmux")
        .args(["set-hook", "-g", "window-linked", &hook_cmd])
        .status()?;
    Command::new("tmux")
        .args(["set-hook", "-g", "window-unlinked", &hook_cmd])
        .status()?;
    Command::new("tmux")
        .args(["set-hook", "-g", "window-renamed", &hook_cmd])
        .status()?;
    Command::new("tmux")
        .args(["set-hook", "-g", "after-select-window", &hook_cmd])
        .status()?;
    Command::new("tmux")
        .args(["set-hook", "-g", "after-new-window", &hook_cmd])
        .status()?;
    Command::new("tmux")
        .args(["set-hook", "-ga", "client-session-changed", &hook_cmd])
        .status()?;

    Ok(())
}
