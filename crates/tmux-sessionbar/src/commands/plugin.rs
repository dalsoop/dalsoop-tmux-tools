use crate::config::template::{self, PluginEntry};
use std::process::Command;

pub fn list() -> Result<(), Box<dyn std::error::Error>> {
    let config = template::load_config()?;

    println!("=== tmux plugins ===\n");
    for plugin in &config.plugins {
        let enabled = plugin.enabled.unwrap_or(true);
        let mark = if enabled { "✓" } else { "✗" };
        let opts = if plugin.options.is_empty() {
            String::new()
        } else {
            format!(" ({})", plugin.options.len())
        };
        println!("  {mark} {}{opts}", plugin.name);
    }

    // Show installed on disk
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let plugin_dir = format!("{home}/.tmux/plugins");
    if let Ok(entries) = std::fs::read_dir(&plugin_dir) {
        let installed: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n != "tpm")
            .collect();

        let config_names: Vec<String> = config.plugins.iter()
            .map(|p| p.name.split('/').last().unwrap_or("").to_string())
            .collect();

        let orphans: Vec<&String> = installed.iter()
            .filter(|i| !config_names.contains(i))
            .collect();

        if !orphans.is_empty() {
            println!("\n  [orphaned on disk]");
            for o in orphans {
                println!("    ? {o}");
            }
        }
    }

    Ok(())
}

pub fn add(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = template::load_config()?;

    // Check if already exists
    if config.plugins.iter().any(|p| p.name == name) {
        // Re-enable if disabled
        for p in &mut config.plugins {
            if p.name == name {
                p.enabled = Some(true);
            }
        }
        println!("enabled: {name}");
    } else {
        config.plugins.push(PluginEntry {
            name: name.to_string(),
            enabled: Some(true),
            options: vec![],
        });
        println!("added: {name}");
    }

    save_config(&config)?;

    // Regenerate .tmux.conf and install
    apply_and_install()?;

    Ok(())
}

pub fn remove(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = template::load_config()?;

    let before = config.plugins.len();
    config.plugins.retain(|p| p.name != name);

    if config.plugins.len() == before {
        println!("not found: {name}");
        return Ok(());
    }

    save_config(&config)?;
    println!("removed: {name}");

    // Regenerate .tmux.conf
    let _ = Command::new("tmux-sessionbar").args(["apply"]).status();

    // Clean orphaned plugin from disk
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let short_name = name.split('/').last().unwrap_or(name);
    let plugin_path = format!("{home}/.tmux/plugins/{short_name}");
    if std::path::Path::new(&plugin_path).exists() {
        std::fs::remove_dir_all(&plugin_path)?;
        println!("cleaned: {plugin_path}");
    }

    Ok(())
}

pub fn install() -> Result<(), Box<dyn std::error::Error>> {
    // Regenerate .tmux.conf first
    let _ = Command::new("tmux-sessionbar").args(["apply"]).status();

    apply_and_install()?;
    Ok(())
}

fn apply_and_install() -> Result<(), Box<dyn std::error::Error>> {
    // Regenerate .tmux.conf
    let _ = Command::new("tmux-sessionbar").args(["apply"]).status();

    // Install via TPM
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let tpm_install = format!("{home}/.tmux/plugins/tpm/bin/install_plugins");

    if std::path::Path::new(&tpm_install).exists() {
        let output = Command::new(&tpm_install).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("success") || line.contains("Already") {
                println!("  {line}");
            }
        }
    } else {
        println!("TPM not found. Install: git clone https://github.com/tmux-plugins/tpm ~/.tmux/plugins/tpm");
    }

    Ok(())
}

fn save_config(config: &template::Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = template::config_path();
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, &content)?;
    Ok(())
}
