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

    // Write click handler script
    let script = format!(r#"#!/bin/bash
RANGE="$1"
rm -f /tmp/tmux-pending-confirm.conf
{binary_path} click "$RANGE" 2>/dev/null || tmux-sessionbar click "$RANGE" 2>/dev/null
"#);
    std::fs::write("/usr/local/bin/tmux-click-handler", &script)?;
    Command::new("chmod").args(["+x", "/usr/local/bin/tmux-click-handler"]).status()?;

    // Mouse binding: run click handler, then check for pending confirm file
    // if-shell checks if the file exists, if so source-file runs confirm-before in tmux context
    Command::new("tmux").args([
        "bind", "-Troot", "MouseDown1Status",
        "if-shell -F '1' \
            \"run-shell '/usr/local/bin/tmux-click-handler \\\"#{mouse_status_range}\\\"'\" ; \
         if-shell 'test -f /tmp/tmux-pending-confirm.conf' \
            'source-file /tmp/tmux-pending-confirm.conf ; run-shell \"rm -f /tmp/tmux-pending-confirm.conf\"'"
    ]).status()?;

    // Trigger sessionbar re-render
    let _ = Command::new("tmux-sessionbar")
        .args(["render-status", "left"])
        .status();

    // Hooks
    let hook_cmd = "run-shell -b 'tmux-sessionbar render-status left'".to_string();
    for hook in &[
        "window-linked", "window-unlinked", "window-renamed",
        "after-select-window", "after-new-window", "after-select-pane", "after-split-window",
    ] {
        Command::new("tmux")
            .args(["set-hook", "-g", hook, &hook_cmd])
            .status()?;
    }
    Command::new("tmux")
        .args(["set-hook", "-ga", "client-session-changed", &hook_cmd])
        .status()?;

    Ok(())
}
