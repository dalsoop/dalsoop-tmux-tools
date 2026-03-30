use anyhow::Result;
use tmux_windowbar::config::template::{Config, config_path, load_config as wbar_load};

pub use wbar_load as load_config;

pub fn save_and_apply(config: &Config) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(config_path(), content)?;
    let _ = std::process::Command::new("tmux-windowbar")
        .args(["apply"])
        .status();
    let _ = std::process::Command::new("tmux-sessionbar")
        .args(["render-status", "left"])
        .status();
    Ok(())
}
