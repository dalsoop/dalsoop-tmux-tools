use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const CONFIG_DIR: &str = ".config/tmux-windowbar";
pub const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct ColorEntry {
    pub fg: String,
    pub bg: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppEntry {
    pub emoji: String,
    pub command: String,
    #[serde(default = "default_app_fg")]
    pub fg: String,
    #[serde(default = "default_app_bg")]
    pub bg: String,
    #[serde(default = "default_app_mode")]
    pub mode: String, // "window" or "pane"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub colors: HashMap<String, ColorEntry>,
    #[serde(default)]
    pub apps: Vec<AppEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    // Line labels
    #[serde(default = "default_users_label")]
    pub users_label: String,
    #[serde(default = "default_windows_label")]
    pub windows_label: String,
    #[serde(default = "default_panes_label")]
    pub panes_label: String,
    #[serde(default = "default_apps_label")]
    pub apps_label: String,
    // View switcher button colors
    #[serde(default = "default_view_all")]
    pub view_all: String,
    #[serde(default = "default_view_user")]
    pub view_user: String,
    #[serde(default = "default_view_session")]
    pub view_session: String,
    #[serde(default = "default_view_compact")]
    pub view_compact: String,
    #[serde(default = "default_view_active_fg")]
    pub view_active_fg: String,
    #[serde(default = "default_view_inactive_fg")]
    pub view_inactive_fg: String,
    #[serde(default = "default_view_inactive_bg")]
    pub view_inactive_bg: String,
    // User states
    #[serde(default = "default_user_viewed_fg")]
    pub user_viewed_fg: String,
    #[serde(default = "default_user_viewed_bg")]
    pub user_viewed_bg: String,
    #[serde(default = "default_user_session_fg")]
    pub user_session_fg: String,
    #[serde(default = "default_user_session_bg")]
    pub user_session_bg: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            users_label: default_users_label(),
            windows_label: default_windows_label(),
            panes_label: default_panes_label(),
            apps_label: default_apps_label(),
            view_all: default_view_all(),
            view_user: default_view_user(),
            view_session: default_view_session(),
            view_compact: default_view_compact(),
            view_active_fg: default_view_active_fg(),
            view_inactive_fg: default_view_inactive_fg(),
            view_inactive_bg: default_view_inactive_bg(),
            user_viewed_fg: default_user_viewed_fg(),
            user_viewed_bg: default_user_viewed_bg(),
            user_session_fg: default_user_session_fg(),
            user_session_bg: default_user_session_bg(),
        }
    }
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
fn default_kill_fg() -> String { "#e06c75".into() }
fn default_kill_bg() -> String { "#282c34".into() }
fn default_button_fg() -> String { "#282c34".into() }
fn default_button_bg() -> String { "#61afef".into() }
fn default_running_fg() -> String { "#282c34".into() }
fn default_running_bg() -> String { "#56b6c2".into() }
fn default_idle_fg() -> String { "#5c6370".into() }
fn default_idle_bg() -> String { "#2c323c".into() }
fn default_app_fg() -> String { "#282c34".into() }
fn default_app_bg() -> String { "#61afef".into() }
fn default_app_mode() -> String { "window".into() }
fn default_users_label() -> String { "#56b6c2".into() }
fn default_windows_label() -> String { "#c678dd".into() }
fn default_panes_label() -> String { "#e5c07b".into() }
fn default_apps_label() -> String { "#e06c75".into() }
fn default_view_all() -> String { "#98c379".into() }
fn default_view_user() -> String { "#61afef".into() }
fn default_view_session() -> String { "#c678dd".into() }
fn default_view_compact() -> String { "#e5c07b".into() }
fn default_view_active_fg() -> String { "#282c34".into() }
fn default_view_inactive_fg() -> String { "#abb2bf".into() }
fn default_view_inactive_bg() -> String { "#3e4452".into() }
fn default_user_viewed_fg() -> String { "#282c34".into() }
fn default_user_viewed_bg() -> String { "#e5c07b".into() }
fn default_user_session_fg() -> String { "#282c34".into() }
fn default_user_session_bg() -> String { "#56b6c2".into() }

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
    let mut colors = HashMap::new();
    colors.insert("spf".into(), ColorEntry { fg: "#282c34".into(), bg: "#c678dd".into() });
    colors.insert("claude".into(), ColorEntry { fg: "#282c34".into(), bg: "#61afef".into() });
    colors.insert("vim".into(), ColorEntry { fg: "#282c34".into(), bg: "#e06c75".into() });
    colors.insert("nvim".into(), ColorEntry { fg: "#282c34".into(), bg: "#e06c75".into() });
    colors.insert("node".into(), ColorEntry { fg: "#282c34".into(), bg: "#98c379".into() });
    colors.insert("python".into(), ColorEntry { fg: "#282c34".into(), bg: "#e5c07b".into() });
    colors.insert("python3".into(), ColorEntry { fg: "#282c34".into(), bg: "#e5c07b".into() });
    colors.insert("htop".into(), ColorEntry { fg: "#282c34".into(), bg: "#d19a66".into() });
    colors.insert("codex".into(), ColorEntry { fg: "#282c34".into(), bg: "#98c379".into() });
    colors.insert("veilkey".into(), ColorEntry { fg: "#282c34".into(), bg: "#98c379".into() });

    let apps = vec![
        AppEntry {
            emoji: "🔐".into(),
            command: "spf".into(),
            fg: "#282c34".into(),
            bg: "#c678dd".into(),
            mode: "window".into(),
        },
        AppEntry {
            emoji: "🤖".into(),
            command: "claude".into(),
            fg: "#282c34".into(),
            bg: "#61afef".into(),
            mode: "window".into(),
        },
        AppEntry {
            emoji: "🧠".into(),
            command: "codex".into(),
            fg: "#282c34".into(),
            bg: "#98c379".into(),
            mode: "window".into(),
        },
        AppEntry {
            emoji: "📊".into(),
            command: "htop".into(),
            fg: "#282c34".into(),
            bg: "#d19a66".into(),
            mode: "window".into(),
        },
        AppEntry {
            emoji: "🔑".into(),
            command: "veilkey".into(),
            fg: "#282c34".into(),
            bg: "#98c379".into(),
            mode: "window".into(),
        },
        AppEntry {
            emoji: "🏛️".into(),
            command: "vaultcenter".into(),
            fg: "#282c34".into(),
            bg: "#56b6c2".into(),
            mode: "window".into(),
        },
        AppEntry {
            emoji: "🏭".into(),
            command: "dalcenter".into(),
            fg: "#282c34".into(),
            bg: "#0f766e".into(),
            mode: "window".into(),
        },
        AppEntry {
            emoji: "🖥️".into(),
            command: "bash".into(),
            fg: "#282c34".into(),
            bg: "#5c6370".into(),
            mode: "window".into(),
        },
    ];

    Config {
        theme: ThemeConfig::default(),
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
        colors,
        apps,
    }
}

pub fn load_config() -> anyhow::Result<Config> {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    } else {
        Ok(default_config())
    }
}
