use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CONFIG_DIR: &str = ".config/tmux-sessionbar";
pub const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginEntry {
    pub name: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_history_limit")]
    pub history_limit: u32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            history_limit: default_history_limit(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    #[serde(default = "default_true")]
    pub auto_clear: bool,
    #[serde(default = "default_clear_interval")]
    pub clear_interval: u32,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            auto_clear: true,
            clear_interval: default_clear_interval(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    pub status: StatusConfig,
    pub blocks: BlocksConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub maintenance: MaintenanceConfig,
    #[serde(default = "default_plugins")]
    pub plugins: Vec<PluginEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusConfig {
    #[serde(default = "default_interval")]
    pub interval: u32,
    #[serde(default = "default_position")]
    pub position: String,
    #[serde(default = "default_bg")]
    pub bg: String,
    #[serde(default = "default_fg")]
    pub fg: String,
    pub left: SegmentConfig,
    pub right: SegmentConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentConfig {
    pub blocks: Vec<String>,
    #[serde(default = "default_length")]
    pub length: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlocksConfig {
    #[serde(rename = "session-list")]
    pub session_list: SessionListBlock,
    #[serde(default)]
    pub hostname: SimpleBlock,
    #[serde(default)]
    pub datetime: DatetimeBlock,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListBlock {
    #[serde(default = "default_active_fg")]
    pub active_fg: String,
    #[serde(default = "default_active_bg")]
    pub active_bg: String,
    #[serde(default = "default_inactive_fg")]
    pub inactive_fg: String,
    #[serde(default = "default_inactive_bg")]
    pub inactive_bg: String,
    #[serde(default = "default_separator")]
    pub separator: String,
    #[serde(default = "default_true")]
    pub show_new_button: bool,
    #[serde(default = "default_true")]
    pub show_kill_button: bool,
    #[serde(default = "default_button_fg")]
    pub button_fg: String,
    #[serde(default = "default_button_bg")]
    pub button_bg: String,
    #[serde(default = "default_kill_bg")]
    pub kill_bg: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleBlock {
    #[serde(default = "default_block_fg")]
    pub fg: String,
    #[serde(default = "default_hostname_bg")]
    pub bg: String,
    #[serde(default = "default_hostname_format")]
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatetimeBlock {
    #[serde(default = "default_block_fg")]
    pub fg: String,
    #[serde(default = "default_datetime_bg")]
    pub bg: String,
    #[serde(default = "default_datetime_format")]
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_true")]
    pub session_switch: bool,
    #[serde(default = "default_true")]
    pub pane_clear: bool,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            session_switch: true,
            pane_clear: true,
        }
    }
}

impl Default for SimpleBlock {
    fn default() -> Self {
        Self {
            fg: default_block_fg(),
            bg: default_hostname_bg(),
            format: default_hostname_format(),
        }
    }
}

impl Default for DatetimeBlock {
    fn default() -> Self {
        Self {
            fg: default_block_fg(),
            bg: default_datetime_bg(),
            format: default_datetime_format(),
        }
    }
}

fn default_interval() -> u32 { 2 }
fn default_position() -> String { "top".into() }
fn default_bg() -> String { "#282c34".into() }
fn default_fg() -> String { "#abb2bf".into() }
fn default_length() -> u32 { 120 }
fn default_active_fg() -> String { "#282c34".into() }
fn default_active_bg() -> String { "#98c379".into() }
fn default_inactive_fg() -> String { "#abb2bf".into() }
fn default_inactive_bg() -> String { "#3e4452".into() }
fn default_separator() -> String { " ".into() }
fn default_block_fg() -> String { "#282c34".into() }
fn default_hostname_bg() -> String { "#61afef".into() }
fn default_hostname_format() -> String { " #H ".into() }
fn default_datetime_bg() -> String { "#c678dd".into() }
fn default_datetime_format() -> String { " %H:%M ".into() }
fn default_true() -> bool { true }
fn default_history_limit() -> u32 { 5000 }
fn default_clear_interval() -> u32 { 30 }
fn default_button_fg() -> String { "#282c34".into() }
fn default_button_bg() -> String { "#61afef".into() }
fn default_kill_bg() -> String { "#e06c75".into() }

fn default_plugins() -> Vec<PluginEntry> {
    vec![
        PluginEntry { name: "tmux-plugins/tmux-resurrect".into(), enabled: Some(true), options: vec![
            "@resurrect-capture-pane-contents 'on'".into(),
        ]},
        PluginEntry { name: "tmux-plugins/tmux-continuum".into(), enabled: Some(true), options: vec![
            "@continuum-restore 'on'".into(),
            "@continuum-save-interval '15'".into(),
        ]},
        PluginEntry { name: "tmux-plugins/tmux-yank".into(), enabled: Some(true), options: vec![] },
        PluginEntry { name: "fcsonline/tmux-thumbs".into(), enabled: Some(true), options: vec![] },
        PluginEntry { name: "tmux-plugins/tmux-open".into(), enabled: Some(true), options: vec![] },
        PluginEntry { name: "tmux-plugins/tmux-logging".into(), enabled: Some(true), options: vec![] },
        PluginEntry { name: "tmux-plugins/tmux-sensible".into(), enabled: Some(true), options: vec![] },
        PluginEntry { name: "rickstaa/tmux-notify".into(), enabled: Some(true), options: vec![] },
        PluginEntry { name: "jaclu/tmux-menus".into(), enabled: Some(true), options: vec![] },
    ]
}

pub fn config_dir() -> PathBuf {
    dirs_home().join(CONFIG_DIR)
}

pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

fn dirs_home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}

pub fn default_config() -> Config {
    Config {
        general: GeneralConfig::default(),
        status: StatusConfig {
            interval: default_interval(),
            position: default_position(),
            bg: default_bg(),
            fg: default_fg(),
            left: SegmentConfig {
                blocks: vec!["session-list".into()],
                length: 120,
            },
            right: SegmentConfig {
                blocks: vec!["hostname".into(), "datetime".into()],
                length: 80,
            },
        },
        blocks: BlocksConfig {
            session_list: SessionListBlock {
                active_fg: default_active_fg(),
                active_bg: default_active_bg(),
                inactive_fg: default_inactive_fg(),
                inactive_bg: default_inactive_bg(),
                separator: default_separator(),
                show_new_button: true,
                show_kill_button: true,
                button_fg: default_button_fg(),
                button_bg: default_button_bg(),
                kill_bg: default_kill_bg(),
            },
            hostname: SimpleBlock::default(),
            datetime: DatetimeBlock::default(),
        },
        keybindings: KeybindingsConfig::default(),
        maintenance: MaintenanceConfig::default(),
        plugins: default_plugins(),
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
