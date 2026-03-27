use crate::config::template;
use anyhow::{Result, bail};
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

    let windowbar_path = std::env::current_exe()?.to_string_lossy().into_owned();
    install_shims(&resolve_executable("tmux-sessionbar")?, &windowbar_path)?;

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

fn install_shims(sessionbar_path: &str, windowbar_path: &str) -> Result<()> {
    let bin_dir = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/root"))
        .join(".config/tmux-sessionbar/bin");
    std::fs::create_dir_all(&bin_dir)?;
    write_shim(&bin_dir.join("tmux-sessionbar"), sessionbar_path)?;
    write_shim(&bin_dir.join("tmux-windowbar"), windowbar_path)?;
    Ok(())
}

fn write_shim(path: &std::path::Path, target: &str) -> Result<()> {
    let script = format!("#!/bin/sh\nexec '{}' \"$@\"\n", shell_escape(target));
    std::fs::write(path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn resolve_executable(name: &str) -> Result<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    bail!("required executable not found in PATH: {name}")
}

fn shell_escape(path: &str) -> String {
    path.replace('\'', "'\"'\"'")
}
