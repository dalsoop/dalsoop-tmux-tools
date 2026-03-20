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

    // Double-click: rename session/window via command-prompt
    // Writes rename command to tmp file, then sources it
    let dblclick_script = format!(r#"#!/bin/bash
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
"#);
    std::fs::write("/usr/local/bin/tmux-dblclick-handler", &dblclick_script)?;
    Command::new("chmod").args(["+x", "/usr/local/bin/tmux-dblclick-handler"]).status()?;

    Command::new("tmux").args([
        "bind", "-Troot", "DoubleClick1Status",
        "if-shell -F '1' \
            \"run-shell '/usr/local/bin/tmux-dblclick-handler \\\"#{mouse_status_range}\\\"'\" ; \
         if-shell 'test -f /tmp/tmux-pending-rename.conf' \
            'source-file /tmp/tmux-pending-rename.conf ; run-shell \"rm -f /tmp/tmux-pending-rename.conf\"'"
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
