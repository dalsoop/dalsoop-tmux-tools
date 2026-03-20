use crate::config::template::{self, default_config};
use crate::config::tmux_conf;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let home = home_dir();
    let tmux_conf_path = home.join(".tmux.conf");
    let config_dir = template::config_dir();
    let config_path = template::config_path();

    // 1. Backup existing .tmux.conf
    if tmux_conf_path.exists() {
        let backup = home.join(".tmux.conf.bak");
        fs::copy(&tmux_conf_path, &backup)?;
        println!("backed up existing .tmux.conf -> .tmux.conf.bak");
    }

    // 2. Create config directory
    fs::create_dir_all(&config_dir)?;
    println!("config dir: {}", config_dir.display());

    // 3. Write default config.toml if not exists
    if !config_path.exists() {
        let config = default_config();
        let content = toml::to_string_pretty(&config)?;
        fs::write(&config_path, &content)?;
        println!("created default config: {}", config_path.display());
    } else {
        println!("config already exists: {}", config_path.display());
    }

    // 4. Generate .tmux.conf
    let config = template::load_config()?;
    let binary_path = std::env::current_exe()?
        .to_string_lossy()
        .to_string();
    let conf_content = tmux_conf::generate(&config, &binary_path);
    fs::write(&tmux_conf_path, &conf_content)?;
    println!("generated: {}", tmux_conf_path.display());

    // 5. Reload tmux if running
    let reload = Command::new("tmux")
        .args(["source-file", &tmux_conf_path.to_string_lossy()])
        .status();

    match reload {
        Ok(s) if s.success() => println!("tmux config reloaded"),
        _ => println!("tmux not running or reload skipped — config will apply on next start"),
    }

    println!("\ndone! session list will appear in the status bar.");
    println!("edit {} to customize blocks.", config_path.display());
    println!("run `tmux-sessionbar apply` after editing config.");

    Ok(())
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}
