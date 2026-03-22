use crate::config::template;
use anyhow::{bail, Context, Result};
use std::process::Command;
use tmux_fmt::tmux;

pub fn run() -> Result<()> {
    let config_path = template::config_path();

    if !config_path.exists() {
        bail!(
            "config not found: {}\nrun `tmux-windowbar init` first.",
            config_path.display()
        );
    }

    apply_settings()?;
    println!("tmux-windowbar applied");
    Ok(())
}

pub fn apply_settings() -> Result<()> {
    // Backfill new fields (e.g. [theme]) into existing config
    let config_path = crate::config::template::config_path();
    if config_path.exists() {
        let config = crate::config::template::load_config()?;
        let updated = toml::to_string_pretty(&config)?;
        std::fs::write(&config_path, &updated)?;
    }

    let binary_path = std::env::current_exe()
        .context("failed to get current exe path")?
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

    tmux::run(&[
        "bind", "-Troot", "MouseDown1Status",
        "if-shell -F '1' \
            \"run-shell '/usr/local/bin/tmux-click-handler \\\"#{mouse_status_range}\\\"'\" ; \
         if-shell 'test -f /tmp/tmux-pending-confirm.conf' \
            'source-file /tmp/tmux-pending-confirm.conf ; run-shell \"rm -f /tmp/tmux-pending-confirm.conf\"'"
    ])?;

    // Double-click: rename session/window via command-prompt
    let dblclick_script = r#"#!/bin/bash
RANGE="$1"
rm -f /tmp/tmux-pending-rename.conf
# Session rename (non-prefixed range = session name)
if echo "$RANGE" | grep -qE '^[a-zA-Z0-9_-]+$' && ! echo "$RANGE" | grep -q '^_'; then
    echo "command-prompt -I \"$RANGE\" -p \"rename session:\" \"rename-session '%%'\"" > /tmp/tmux-pending-rename.conf
# Window rename (_ws or _wa prefix)
elif echo "$RANGE" | grep -qE '^_ws'; then
    IDX=$(echo "$RANGE" | sed 's/^_ws//')
    echo "command-prompt -p \"rename window $IDX:\" \"rename-window -t :$IDX '%%'\"" > /tmp/tmux-pending-rename.conf
elif echo "$RANGE" | grep -qE '^_wa'; then
    TARGET=$(echo "$RANGE" | sed 's/^_wa//')
    SESS=$(echo "$TARGET" | cut -d. -f1)
    WIN=$(echo "$TARGET" | cut -d. -f2)
    echo "command-prompt -p \"rename window $SESS:$WIN:\" \"rename-window -t =$SESS:$WIN '%%'\"" > /tmp/tmux-pending-rename.conf
fi
"#;
    std::fs::write("/usr/local/bin/tmux-dblclick-handler", dblclick_script)?;
    Command::new("chmod").args(["+x", "/usr/local/bin/tmux-dblclick-handler"]).status()?;

    tmux::run(&[
        "bind", "-Troot", "DoubleClick1Status",
        "if-shell -F '1' \
            \"run-shell '/usr/local/bin/tmux-dblclick-handler \\\"#{mouse_status_range}\\\"'\" ; \
         if-shell 'test -f /tmp/tmux-pending-rename.conf' \
            'source-file /tmp/tmux-pending-rename.conf ; run-shell \"rm -f /tmp/tmux-pending-rename.conf\"'"
    ])?;

    // Trigger sessionbar re-render
    let _ = Command::new("tmux-sessionbar")
        .args(["render-status", "left"])
        .status();

    // Hooks
    let hook_cmd = "run-shell -b 'tmux-sessionbar render-status left'";
    for hook in &[
        "window-linked", "window-unlinked", "window-renamed",
        "after-select-window", "after-new-window", "after-select-pane", "after-split-window",
    ] {
        tmux::run(&["set-hook", "-g", hook, hook_cmd])?;
    }
    tmux::run(&["set-hook", "-ga", "client-session-changed", hook_cmd])?;

    Ok(())
}
