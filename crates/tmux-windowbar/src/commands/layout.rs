use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn layout_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(home).join(".config/tmux-windowbar/layouts")
}

pub fn save(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = layout_dir();
    fs::create_dir_all(&dir)?;

    // Save each window's layout string
    let output = Command::new("tmux")
        .args(["list-windows", "-F", "#{window_index}:#{window_name}:#{window_layout}"])
        .output()?;
    let content = String::from_utf8_lossy(&output.stdout).to_string();

    let path = dir.join(format!("{name}.layout"));
    fs::write(&path, &content)?;
    println!("saved layout '{name}' ({} windows)", content.lines().count());

    Ok(())
}

pub fn load(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = layout_dir().join(format!("{name}.layout"));
    if !path.exists() {
        return Err(format!("layout '{name}' not found").into());
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
        let check = Command::new("tmux")
            .args(["select-window", "-t", &format!(":{idx}")])
            .status();
        if !check.map(|s| s.success()).unwrap_or(false) {
            Command::new("tmux")
                .args(["new-window", "-t", &format!(":{idx}")])
                .status()?;
        }

        // Rename window
        Command::new("tmux")
            .args(["rename-window", "-t", &format!(":{idx}"), win_name])
            .status()?;

        // Apply layout
        if !layout.is_empty() {
            Command::new("tmux")
                .args(["select-layout", "-t", &format!(":{idx}"), layout])
                .status()?;
        }
    }

    println!("loaded layout '{name}'");
    Ok(())
}

pub fn list() -> Result<(), Box<dyn std::error::Error>> {
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
