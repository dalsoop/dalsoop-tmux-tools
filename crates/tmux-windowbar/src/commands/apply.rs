use crate::config::template;
use anyhow::{Result, bail};
use std::process::Command;
use tmux_fmt::shims;
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

    let windowbar_path = std::env::current_exe()?.to_string_lossy().into_owned();
    let bin_dir = tmux::home_dir().join(".config/tmux-sessionbar/bin");
    shims::install_shims(
        &bin_dir,
        &shims::resolve_executable("tmux-sessionbar")?,
        &windowbar_path,
    )?;

    tmux::run(&[
        "bind",
        "-Troot",
        "MouseDown1Status",
        "run-shell '$HOME/.config/tmux-sessionbar/bin/tmux-windowbar click \"#{mouse_status_range}\" 2>/dev/null || $HOME/.config/tmux-sessionbar/bin/tmux-sessionbar click \"#{mouse_status_range}\" 2>/dev/null'",
    ])?;

    tmux::run(&[
        "bind",
        "-Troot",
        "DoubleClick1Status",
        "run-shell '$HOME/.config/tmux-sessionbar/bin/tmux-windowbar dblclick \"#{mouse_status_range}\" 2>/dev/null'",
    ])?;

    // Trigger sessionbar re-render
    let _ = Command::new("tmux-sessionbar")
        .args(["render-status", "left"])
        .status();

    // Hooks
    let hook_cmd = "run-shell -b 'tmux-sessionbar render-status left'";
    for hook in &["window-linked", "window-unlinked", "window-renamed"] {
        tmux::run(&["set-hook", "-g", hook, hook_cmd])?;
    }

    Ok(())
}
