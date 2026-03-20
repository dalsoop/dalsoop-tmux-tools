use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CONFIG_DIR: &str = ".config/tmux-windowbar";
pub const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    #[serde(default = "default_true")]
    pub show_kill_button: bool,
    #[serde(default = "default_true")]
    pub show_new_button: bool,
    #[serde(default = "default_fg")]
    pub fg: String,
    #[serde(default = "default_bg")]
    pub bg: String,
    #[serde(default = "default_active_fg")]
    pub active_fg: String,
    #[serde(default = "default_active_bg")]
    pub active_bg: String,
    #[serde(default = "default_kill_fg")]
    pub kill_fg: String,
    #[serde(default = "default_kill_bg")]
    pub kill_bg: String,
    #[serde(default = "default_button_fg")]
    pub button_fg: String,
    #[serde(default = "default_button_bg")]
    pub button_bg: String,
    #[serde(default = "default_running_fg")]
    pub running_fg: String,
    #[serde(default = "default_running_bg")]
    pub running_bg: String,
    #[serde(default = "default_idle_fg")]
    pub idle_fg: String,
    #[serde(default = "default_idle_bg")]
    pub idle_bg: String,
}

fn default_true() -> bool { true }
fn default_fg() -> String { "#abb2bf".into() }
fn default_bg() -> String { "#282c34".into() }
fn default_active_fg() -> String { "#282c34".into() }
fn default_active_bg() -> String { "#98c379".into() }
fn default_kill_fg() -> String { "#282c34".into() }
fn default_kill_bg() -> String { "#e06c75".into() }
fn default_button_fg() -> String { "#282c34".into() }
fn default_button_bg() -> String { "#61afef".into() }
fn default_running_fg() -> String { "#282c34".into() }
fn default_running_bg() -> String { "#56b6c2".into() }
fn default_idle_fg() -> String { "#5c6370".into() }
fn default_idle_bg() -> String { "#2c323c".into() }

pub fn config_dir() -> PathBuf {
    home_dir().join(CONFIG_DIR)
}

pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}

pub fn default_config() -> Config {
    Config {
        window: WindowConfig {
            show_kill_button: true,
            show_new_button: true,
            fg: default_fg(),
            bg: default_bg(),
            active_fg: default_active_fg(),
            active_bg: default_active_bg(),
            kill_fg: default_kill_fg(),
            kill_bg: default_kill_bg(),
            button_fg: default_button_fg(),
            button_bg: default_button_bg(),
            running_fg: default_running_fg(),
            running_bg: default_running_bg(),
            idle_fg: default_idle_fg(),
            idle_bg: default_idle_bg(),
        },
    }
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    } else {
        Ok(default_config())
    }
}
