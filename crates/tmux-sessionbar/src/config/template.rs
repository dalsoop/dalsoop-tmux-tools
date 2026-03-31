use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tmux_fmt::theme::*;
use tmux_fmt::tmux;

pub const CONFIG_DIR: &str = ".config/tmux-sessionbar";
pub const CONFIG_FILE: &str = "config.toml";
pub const BIN_DIR: &str = "bin";

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
    pub theme: ThemeConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub pane_border: PaneBorderConfig,
    #[serde(default)]
    pub maintenance: MaintenanceConfig,
    #[serde(default = "default_plugins")]
    pub plugins: Vec<PluginEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_label_fg")]
    pub label_fg: String,
    #[serde(default = "default_stats_fg")]
    pub stats_fg: String,
    #[serde(default = "default_stats_bg")]
    pub stats_bg: String,
    #[serde(default = "default_mem_fg")]
    pub mem_fg: String,
    #[serde(default = "default_mem_normal")]
    pub mem_normal: String,
    #[serde(default = "default_mem_warn")]
    pub mem_warn: String,
    #[serde(default = "default_mem_critical")]
    pub mem_critical: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            label_fg: default_label_fg(),
            stats_fg: default_stats_fg(),
            stats_bg: default_stats_bg(),
            mem_fg: default_mem_fg(),
            mem_normal: default_mem_normal(),
            mem_warn: default_mem_warn(),
            mem_critical: default_mem_critical(),
        }
    }
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
    #[serde(default = "default_kill_fg", alias = "kill_bg")]
    pub kill_fg: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PaneBorderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_pane_border_active_fg")]
    pub active_fg: String,
    #[serde(default = "default_pane_border_active_bg")]
    pub active_bg: String,
    #[serde(default = "default_pane_border_inactive_fg")]
    pub inactive_fg: String,
    #[serde(default = "default_pane_border_inactive_bg")]
    pub inactive_bg: String,
}

impl Default for PaneBorderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            active_fg: default_pane_border_active_fg(),
            active_bg: default_pane_border_active_bg(),
            inactive_fg: default_pane_border_inactive_fg(),
            inactive_bg: default_pane_border_inactive_bg(),
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

fn default_interval() -> u32 {
    2
}
fn default_position() -> String {
    "top".into()
}
fn default_length() -> u32 {
    120
}
fn default_inactive_fg() -> String {
    "#abb2bf".into()
}
fn default_inactive_bg() -> String {
    "#3e4452".into()
}
fn default_separator() -> String {
    " ".into()
}
fn default_block_fg() -> String {
    "#282c34".into()
}
fn default_hostname_bg() -> String {
    "#61afef".into()
}
fn default_hostname_format() -> String {
    " #H ".into()
}
fn default_datetime_bg() -> String {
    "#c678dd".into()
}
fn default_datetime_format() -> String {
    " %H:%M ".into()
}
fn default_history_limit() -> u32 {
    5000
}
fn default_clear_interval() -> u32 {
    30
}
fn default_label_fg() -> String {
    "#98c379".into()
}
fn default_stats_fg() -> String {
    "#abb2bf".into()
}
fn default_stats_bg() -> String {
    "#3e4452".into()
}
fn default_mem_fg() -> String {
    "#282c34".into()
}
fn default_mem_normal() -> String {
    "#98c379".into()
}
fn default_mem_warn() -> String {
    "#e5c07b".into()
}
fn default_mem_critical() -> String {
    "#e06c75".into()
}

fn default_pane_border_active_fg() -> String {
    "#282c34".into()
}
fn default_pane_border_active_bg() -> String {
    "#98c379".into()
}
fn default_pane_border_inactive_fg() -> String {
    "#5c6370".into()
}
fn default_pane_border_inactive_bg() -> String {
    "#282c34".into()
}

fn default_plugins() -> Vec<PluginEntry> {
    vec![
        PluginEntry {
            name: "tmux-plugins/tmux-resurrect".into(),
            enabled: Some(true),
            options: vec!["@resurrect-capture-pane-contents 'on'".into()],
        },
        PluginEntry {
            name: "tmux-plugins/tmux-continuum".into(),
            enabled: Some(true),
            options: vec![
                "@continuum-restore 'on'".into(),
                "@continuum-save-interval '15'".into(),
            ],
        },
        PluginEntry {
            name: "tmux-plugins/tmux-yank".into(),
            enabled: Some(true),
            options: vec![],
        },
        PluginEntry {
            name: "fcsonline/tmux-thumbs".into(),
            enabled: Some(true),
            options: vec![],
        },
        PluginEntry {
            name: "tmux-plugins/tmux-open".into(),
            enabled: Some(true),
            options: vec![],
        },
        PluginEntry {
            name: "tmux-plugins/tmux-logging".into(),
            enabled: Some(true),
            options: vec![],
        },
        PluginEntry {
            name: "tmux-plugins/tmux-sensible".into(),
            enabled: Some(true),
            options: vec![],
        },
        PluginEntry {
            name: "rickstaa/tmux-notify".into(),
            enabled: Some(true),
            options: vec![],
        },
        PluginEntry {
            name: "jaclu/tmux-menus".into(),
            enabled: Some(true),
            options: vec![],
        },
    ]
}

pub fn config_dir() -> PathBuf {
    tmux::home_dir().join(CONFIG_DIR)
}

pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

pub fn bin_dir() -> PathBuf {
    config_dir().join(BIN_DIR)
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
                kill_fg: default_kill_fg(),
            },
            hostname: SimpleBlock::default(),
            datetime: DatetimeBlock::default(),
        },
        theme: ThemeConfig::default(),
        keybindings: KeybindingsConfig::default(),
        pane_border: PaneBorderConfig::default(),
        maintenance: MaintenanceConfig::default(),
        plugins: default_plugins(),
    }
}

pub fn load_config() -> anyhow::Result<Config> {
    tmux::load_toml_config(&config_path(), default_config)
}
