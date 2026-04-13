use anyhow::Result;
use tmux_windowbar::config::template::{Config as WbConfig, config_path as wb_config_path};

pub fn load_wb_config() -> Result<WbConfig> {
    tmux_windowbar::config::template::load_config()
}

pub fn save_and_apply_wb(config: &WbConfig) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(wb_config_path(), content)?;
    let _ = std::process::Command::new("tmux-windowbar")
        .args(["apply"])
        .status();
    let _ = std::process::Command::new("tmux-sessionbar")
        .args(["render-status", "left"])
        .status();
    Ok(())
}

pub fn load_sb_config() -> Result<tmux_sessionbar::config::template::Config> {
    tmux_sessionbar::config::template::load_config()
}

pub fn save_and_apply_sb(config: &tmux_sessionbar::config::template::Config) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(tmux_sessionbar::config::template::config_path(), content)?;
    let _ = std::process::Command::new("tmux-sessionbar")
        .args(["apply"])
        .status();
    Ok(())
}

// Backward compat aliases
pub fn load_config() -> Result<WbConfig> {
    load_wb_config()
}

pub fn save_and_apply(config: &WbConfig) -> Result<()> {
    save_and_apply_wb(config)
}
