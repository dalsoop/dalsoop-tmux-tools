use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tmux_fmt::theme::*;
use tmux_fmt::tmux;

pub const CONFIG_DIR: &str = ".config/tmux-windowbar";
pub const CONFIG_FILE: &str = "config.toml";
pub const APPS_D_DIR: &str = "apps.d";

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
    /// "window" or "pane". 비어 있으면 `WindowConfig::default_app_mode` 를 따름.
    /// `effective_mode()` 로 조회 권장.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl AppEntry {
    /// 실제 적용될 mode — `mode` 명시값 또는 `WindowConfig::default_app_mode` fallback.
    pub fn effective_mode<'a>(&'a self, window: &'a WindowConfig) -> &'a str {
        self.mode.as_deref().unwrap_or(window.default_app_mode.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SshEntry {
    pub name: String,
    pub host: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default = "default_ssh_emoji")]
    pub emoji: String,
    #[serde(default = "default_ssh_fg")]
    pub fg: String,
    #[serde(default = "default_ssh_bg")]
    pub bg: String,
    /// "ssh" (default), "proxmox" (SSH-based), or "proxmox-api" (REST API)
    #[serde(default = "default_ssh_type")]
    pub r#type: String,
    /// Password for API auth (proxmox-api type only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// API port (default 8006, for proxmox-api type)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

fn default_ssh_type() -> String {
    "ssh".into()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub colors: HashMap<String, ColorEntry>,
    /// `config.toml` 안의 인라인 `[[apps]]` 항목. apply 시 다시 직렬화됨.
    #[serde(default)]
    pub apps: Vec<AppEntry>,
    /// `apps.d/*.toml` 모듈에서 로드된 항목. 직렬화/저장 안 됨 (read-only).
    #[serde(skip)]
    pub modular_apps: Vec<AppEntry>,
    #[serde(default)]
    pub ssh: Vec<SshEntry>,
}

impl Config {
    /// 인라인 + 모듈식 apps 를 합친 iterator. render/click 등 사용자에게 노출되는
    /// 모든 앱 순회는 이 함수를 사용해야 한다 (인덱스 일관성 보장).
    pub fn all_apps(&self) -> impl Iterator<Item = &AppEntry> {
        self.apps.iter().chain(self.modular_apps.iter())
    }
}

/// `apps.d/*.toml` 한 파일의 형식.
#[derive(Debug, Deserialize, Default)]
struct AppsModule {
    #[serde(default)]
    apps: Vec<AppEntry>,
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
    // User states
    #[serde(default = "default_user_viewed_fg")]
    pub user_viewed_fg: String,
    #[serde(default = "default_user_viewed_bg")]
    pub user_viewed_bg: String,
    #[serde(default = "default_user_session_fg")]
    pub user_session_fg: String,
    #[serde(default = "default_user_session_bg")]
    pub user_session_bg: String,
    // SSH connected state
    #[serde(default = "default_ssh_connected_fg")]
    pub ssh_connected_fg: String,
    #[serde(default = "default_ssh_connected_bg")]
    pub ssh_connected_bg: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            users_label: default_users_label(),
            windows_label: default_windows_label(),
            panes_label: default_panes_label(),
            apps_label: default_apps_label(),
            user_viewed_fg: default_user_viewed_fg(),
            user_viewed_bg: default_user_viewed_bg(),
            user_session_fg: default_user_session_fg(),
            user_session_bg: default_user_session_bg(),
            ssh_connected_fg: default_ssh_connected_fg(),
            ssh_connected_bg: default_ssh_connected_bg(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    #[serde(default = "default_true")]
    pub show_kill_button: bool,
    #[serde(default = "default_true")]
    pub show_new_button: bool,
    /// Default mode for new apps: "window" or "pane"
    #[serde(default = "default_app_mode")]
    pub default_app_mode: String,
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

fn default_kill_bg() -> String {
    "#282c34".into()
}
fn default_running_fg() -> String {
    "#282c34".into()
}
fn default_running_bg() -> String {
    "#56b6c2".into()
}
fn default_idle_fg() -> String {
    "#5c6370".into()
}
fn default_idle_bg() -> String {
    "#2c323c".into()
}
fn default_app_fg() -> String {
    "#282c34".into()
}
fn default_app_bg() -> String {
    "#61afef".into()
}
fn default_app_mode() -> String {
    "pane".into()
}
fn default_users_label() -> String {
    "#56b6c2".into()
}
fn default_windows_label() -> String {
    "#c678dd".into()
}
fn default_panes_label() -> String {
    "#e5c07b".into()
}
fn default_apps_label() -> String {
    "#e06c75".into()
}
fn default_user_viewed_fg() -> String {
    "#282c34".into()
}
fn default_user_viewed_bg() -> String {
    "#e5c07b".into()
}
fn default_user_session_fg() -> String {
    "#282c34".into()
}
fn default_user_session_bg() -> String {
    "#56b6c2".into()
}
fn default_ssh_emoji() -> String {
    "\u{1f5a5}\u{fe0f}".into()
}
fn default_ssh_fg() -> String {
    "#abb2bf".into()
}
fn default_ssh_bg() -> String {
    "#3e4452".into()
}
fn default_ssh_connected_fg() -> String {
    "#282c34".into()
}
fn default_ssh_connected_bg() -> String {
    "#98c379".into()
}

pub fn config_dir() -> PathBuf {
    tmux::home_dir().join(CONFIG_DIR)
}

pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

/// `apps.d/` 디렉토리. 여기 있는 모든 `*.toml` 파일이 모듈식 앱으로 로드됨.
pub fn apps_d_path() -> PathBuf {
    config_dir().join(APPS_D_DIR)
}

/// 디렉토리에서 `*.toml` 파일을 알파벳 순으로 읽어 `AppEntry` 들을 모은다.
/// 파싱 실패는 stderr 경고만 출력하고 무시 — 한 모듈이 깨져도 나머지 살아남아야 함.
pub fn load_modular_apps(dir: &std::path::Path) -> Vec<AppEntry> {
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
            .collect(),
        Err(_) => return Vec::new(),
    };
    entries.sort();

    let mut apps = Vec::new();
    for path in entries {
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<AppsModule>(&content) {
                Ok(module) => apps.extend(module.apps),
                Err(e) => eprintln!(
                    "[tmux-windowbar] {} 파싱 실패 (스킵): {e}",
                    path.display()
                ),
            },
            Err(e) => eprintln!(
                "[tmux-windowbar] {} 읽기 실패 (스킵): {e}",
                path.display()
            ),
        }
    }
    apps
}

impl Default for Config {
    fn default() -> Self {
        default_config()
    }
}

pub fn default_config() -> Config {
    let mut colors = HashMap::new();
    colors.insert(
        "spf".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#c678dd".into(),
        },
    );
    colors.insert(
        "claude".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#61afef".into(),
        },
    );
    colors.insert(
        "vim".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#e06c75".into(),
        },
    );
    colors.insert(
        "nvim".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#e06c75".into(),
        },
    );
    colors.insert(
        "node".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#98c379".into(),
        },
    );
    colors.insert(
        "python".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#e5c07b".into(),
        },
    );
    colors.insert(
        "python3".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#e5c07b".into(),
        },
    );
    colors.insert(
        "htop".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#d19a66".into(),
        },
    );
    colors.insert(
        "codex".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#98c379".into(),
        },
    );
    colors.insert(
        "lazygit".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#e06c75".into(),
        },
    );
    colors.insert(
        "lazydocker".into(),
        ColorEntry {
            fg: "#282c34".into(),
            bg: "#61afef".into(),
        },
    );

    let apps = vec![
        AppEntry {
            emoji: "📊".into(),
            command: "htop".into(),
            fg: "#282c34".into(),
            bg: "#d19a66".into(),
            mode: None,
        },
        AppEntry {
            emoji: "📂".into(),
            command: "lazygit".into(),
            fg: "#282c34".into(),
            bg: "#e06c75".into(),
            mode: None,
        },
        AppEntry {
            emoji: "🐳".into(),
            command: "lazydocker".into(),
            fg: "#282c34".into(),
            bg: "#61afef".into(),
            mode: None,
        },
        AppEntry {
            emoji: "🖥️".into(),
            command: "bash".into(),
            fg: "#282c34".into(),
            bg: "#5c6370".into(),
            mode: None,
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
            default_app_mode: default_app_mode(),
        },
        colors,
        apps,
        modular_apps: Vec::new(),
        ssh: vec![],
    }
}

pub fn load_config() -> anyhow::Result<Config> {
    let mut config: Config = tmux::load_toml_config(&config_path(), default_config)?;
    config.modular_apps = load_modular_apps(&apps_d_path());
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_apps_contains_htop() {
        let config = default_config();
        assert!(
            config.apps.iter().any(|app| app.command == "htop"),
            "default apps should contain htop"
        );
    }

    #[test]
    fn default_config_has_no_ssh() {
        let config = default_config();
        assert!(config.ssh.is_empty(), "default config should have no SSH entries");
    }

    #[test]
    fn default_app_mode_is_pane() {
        let config = default_config();
        assert_eq!(config.window.default_app_mode, "pane");
    }

    #[test]
    fn default_apps_have_no_explicit_mode() {
        // 기본 앱들은 mode = None 으로 두어 글로벌 default_app_mode 를 따라야 함
        let config = default_config();
        for app in &config.apps {
            assert!(
                app.mode.is_none(),
                "default app '{}' should not pin its mode",
                app.command
            );
        }
    }

    #[test]
    fn effective_mode_falls_back_to_window_default() {
        let config = default_config();
        let app = &config.apps[0];
        assert_eq!(app.effective_mode(&config.window), "pane");
    }

    #[test]
    fn effective_mode_uses_explicit_value_when_set() {
        let config = default_config();
        let app = AppEntry {
            emoji: "X".into(),
            command: "x".into(),
            fg: "#000".into(),
            bg: "#fff".into(),
            mode: Some("window".into()),
        };
        assert_eq!(app.effective_mode(&config.window), "window");
    }

    #[test]
    fn all_apps_chains_inline_then_modular() {
        let mut config = default_config();
        let inline_count = config.apps.len();
        config.modular_apps.push(AppEntry {
            emoji: "🏭".into(),
            command: "modular-1".into(),
            fg: "#000".into(),
            bg: "#fff".into(),
            mode: None,
        });
        config.modular_apps.push(AppEntry {
            emoji: "🏭".into(),
            command: "modular-2".into(),
            fg: "#000".into(),
            bg: "#fff".into(),
            mode: None,
        });
        let all: Vec<_> = config.all_apps().collect();
        assert_eq!(all.len(), inline_count + 2);
        // 인라인이 먼저, 모듈이 그 다음
        assert_eq!(all[inline_count].command, "modular-1");
        assert_eq!(all[inline_count + 1].command, "modular-2");
    }

    #[test]
    fn load_modular_apps_returns_empty_when_dir_missing() {
        let path = std::path::Path::new("/tmp/tmux-windowbar-test-no-such-dir-xyz");
        let _ = std::fs::remove_dir_all(path);
        let apps = load_modular_apps(path);
        assert!(apps.is_empty());
    }

    #[test]
    fn load_modular_apps_reads_alphabetic_order() {
        let dir = std::env::temp_dir().join("tmux-windowbar-test-apps-d");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("20-second.toml"),
            "[[apps]]\nemoji = \"B\"\ncommand = \"second\"\n",
        ).unwrap();
        std::fs::write(
            dir.join("10-first.toml"),
            "[[apps]]\nemoji = \"A\"\ncommand = \"first\"\n",
        ).unwrap();
        // 비-toml 파일은 무시
        std::fs::write(dir.join("README.md"), "ignored").unwrap();

        let apps = load_modular_apps(&dir);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].command, "first");
        assert_eq!(apps[1].command, "second");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_modular_apps_skips_broken_files_quietly() {
        let dir = std::env::temp_dir().join("tmux-windowbar-test-apps-d-broken");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("00-good.toml"),
            "[[apps]]\nemoji = \"A\"\ncommand = \"alive\"\n",
        ).unwrap();
        std::fs::write(dir.join("10-broken.toml"), "this is not valid toml = ===").unwrap();
        std::fs::write(
            dir.join("20-also-good.toml"),
            "[[apps]]\nemoji = \"C\"\ncommand = \"survivor\"\n",
        ).unwrap();

        let apps = load_modular_apps(&dir);
        assert_eq!(apps.len(), 2, "broken file should be skipped, others survive");
        assert_eq!(apps[0].command, "alive");
        assert_eq!(apps[1].command, "survivor");

        std::fs::remove_dir_all(&dir).ok();
    }
}
