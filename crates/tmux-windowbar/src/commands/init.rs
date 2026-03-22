use crate::config::template::{self, default_config};
use anyhow::Result;
use std::fs;

pub fn run() -> Result<()> {
    let config_dir = template::config_dir();
    let config_path = template::config_path();

    fs::create_dir_all(&config_dir)?;
    println!("config dir: {}", config_dir.display());

    if !config_path.exists() {
        let config = default_config();
        let content = toml::to_string_pretty(&config)?;
        fs::write(&config_path, &content)?;
        println!("created default config: {}", config_path.display());
    } else {
        println!("config already exists: {}", config_path.display());
    }

    super::apply::apply_settings()?;

    println!("\ndone! window [+][x] buttons active.");
    println!("edit {} to customize.", config_path.display());
    println!("run `tmux-windowbar apply` after editing config.");

    Ok(())
}
