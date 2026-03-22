use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;
use tmux_fmt::tmux;

fn layout_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(home).join(".config/tmux-windowbar/layouts")
}

pub fn save(name: &str) -> Result<()> {
    let dir = layout_dir();
    fs::create_dir_all(&dir)?;

    let content = tmux::query(&[
        "list-windows", "-F", "#{window_index}:#{window_name}:#{window_layout}",
    ])?;

    let path = dir.join(format!("{name}.layout"));
    fs::write(&path, &content)?;
    println!("saved layout '{name}' ({} windows)", content.lines().count());

    Ok(())
}

pub fn load(name: &str) -> Result<()> {
    let path = layout_dir().join(format!("{name}.layout"));
    if !path.exists() {
        bail!("layout '{name}' not found");
    }

    let content = fs::read_to_string(&path)?;

    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, ':');
        let idx = parts.next().unwrap_or("");
        let win_name = parts.next().unwrap_or("");
        let layout = parts.next().unwrap_or("");

        // Create window if it doesn't exist
        if tmux::run(&["select-window", "-t", &format!(":{idx}")]).is_err() {
            tmux::run(&["new-window", "-t", &format!(":{idx}")])?;
        }

        tmux::run(&["rename-window", "-t", &format!(":{idx}"), win_name])?;

        if !layout.is_empty() {
            tmux::run(&["select-layout", "-t", &format!(":{idx}"), layout])?;
        }
    }

    println!("loaded layout '{name}'");
    Ok(())
}

pub fn list() -> Result<()> {
    let dir = layout_dir();
    if !dir.exists() {
        println!("no saved layouts");
        return Ok(());
    }

    let mut found = false;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".layout") {
            let layout_name = name.trim_end_matches(".layout");
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            let windows = content.lines().count();
            println!("  {layout_name} ({windows} windows)");
            found = true;
        }
    }

    if !found {
        println!("no saved layouts");
    }

    Ok(())
}
