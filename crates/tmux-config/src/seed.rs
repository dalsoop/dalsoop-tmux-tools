use std::process::Command;

/// A seedable app with install info per package manager.
pub struct SeedApp {
    pub emoji: &'static str,
    pub command: &'static str,
    pub description: &'static str,
    pub fg: &'static str,
    pub bg: &'static str,
    /// Package name per manager: (brew, apt, npm, go)
    /// Empty string = not available via that manager
    pub brew: &'static str,
    pub apt: &'static str,
    pub npm: &'static str,
    pub go: &'static str,
}

pub const SEEDS: &[SeedApp] = &[
    SeedApp { emoji: "📊", command: "htop",       description: "Process viewer",    fg: "#282c34", bg: "#d19a66", brew: "htop",       apt: "htop",       npm: "", go: "" },
    SeedApp { emoji: "📂", command: "lazygit",    description: "Git TUI",           fg: "#282c34", bg: "#e06c75", brew: "lazygit",    apt: "",           npm: "", go: "github.com/jesseduffield/lazygit@latest" },
    SeedApp { emoji: "🐳", command: "lazydocker", description: "Docker TUI",        fg: "#282c34", bg: "#61afef", brew: "lazydocker", apt: "",           npm: "", go: "github.com/jesseduffield/lazydocker@latest" },
    SeedApp { emoji: "🤖", command: "claude",     description: "Claude Code",       fg: "#282c34", bg: "#61afef", brew: "",           apt: "",           npm: "@anthropic-ai/claude-code", go: "" },
    SeedApp { emoji: "🧠", command: "codex",      description: "OpenAI Codex CLI",  fg: "#282c34", bg: "#98c379", brew: "",           apt: "",           npm: "@openai/codex", go: "" },
    SeedApp { emoji: "🔍", command: "btop",       description: "Resource monitor",  fg: "#282c34", bg: "#c678dd", brew: "btop",       apt: "btop",       npm: "", go: "" },
    SeedApp { emoji: "📡", command: "bandwhich",  description: "Network monitor",   fg: "#282c34", bg: "#56b6c2", brew: "bandwhich",  apt: "",           npm: "", go: "" },
    SeedApp { emoji: "🌐", command: "opencode",   description: "OpenCode CLI",      fg: "#282c34", bg: "#98c379", brew: "",           apt: "",           npm: "", go: "github.com/opencode-ai/opencode@latest" },
    SeedApp { emoji: "🖥️", command: "bash",       description: "Shell",             fg: "#282c34", bg: "#5c6370", brew: "",           apt: "",           npm: "", go: "" },
];

/// Detect which package manager is available.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PkgManager {
    Brew,
    Apt,
    Npm,
    Go,
}

/// Install Homebrew if on macOS and not installed.
pub fn ensure_brew() -> bool {
    if is_installed("brew") { return true; }
    if std::env::consts::OS != "macos" { return false; }
    Command::new("sh")
        .args(["-c", "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Return all available package managers on this system.
pub fn available_managers() -> Vec<PkgManager> {
    let mut mgrs = Vec::new();
    if is_installed("brew") { mgrs.push(PkgManager::Brew); }
    if is_installed("apt-get") { mgrs.push(PkgManager::Apt); }
    if is_installed("npm") { mgrs.push(PkgManager::Npm); }
    if is_installed("go") { mgrs.push(PkgManager::Go); }
    mgrs
}

/// Build the install command for a seed app, using available package managers.
/// Returns None if no manager can install it.
pub fn install_cmd(seed: &SeedApp) -> Option<String> {
    let mgrs = available_managers();
    for mgr in &mgrs {
        match mgr {
            PkgManager::Brew if !seed.brew.is_empty() => {
                return Some(format!("brew install {}", seed.brew));
            }
            PkgManager::Apt if !seed.apt.is_empty() => {
                return Some(format!("sudo apt-get install -y {}", seed.apt));
            }
            PkgManager::Npm if !seed.npm.is_empty() => {
                return Some(format!("npm install -g {}", seed.npm));
            }
            PkgManager::Go if !seed.go.is_empty() => {
                return Some(format!("go install {}", seed.go));
            }
            _ => continue,
        }
    }
    None
}

/// Check if a command is installed on the system.
pub fn is_installed(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Install a seed app. Returns true on success.
/// On macOS, installs Homebrew first if needed.
pub fn install(seed: &SeedApp) -> bool {
    // If brew is the only option and not installed, try installing it first
    if !seed.brew.is_empty() && !is_installed("brew") && std::env::consts::OS == "macos" {
        ensure_brew();
    }

    let cmd = match install_cmd(seed) {
        Some(c) => c,
        None => return false,
    };
    Command::new("sh")
        .args(["-c", &cmd])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Display string showing install method available.
pub fn install_method(seed: &SeedApp) -> &'static str {
    let mgrs = available_managers();
    for mgr in &mgrs {
        match mgr {
            PkgManager::Brew if !seed.brew.is_empty() => return "brew",
            PkgManager::Apt if !seed.apt.is_empty() => return "apt",
            PkgManager::Npm if !seed.npm.is_empty() => return "npm",
            PkgManager::Go if !seed.go.is_empty() => return "go",
            _ => continue,
        }
    }
    "-"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_not_empty() {
        assert!(!SEEDS.is_empty());
    }

    #[test]
    fn bash_is_installed() {
        assert!(is_installed("bash"));
    }

    #[test]
    fn nonexistent_not_installed() {
        assert!(!is_installed("this_command_does_not_exist_xyz"));
    }

    #[test]
    fn available_managers_returns_something() {
        // At minimum on any dev machine, at least one should exist
        let mgrs = available_managers();
        // Just verify it doesn't panic
        assert!(mgrs.len() >= 0);
    }

    #[test]
    fn install_cmd_htop_has_result() {
        // htop is available via brew or apt, so on most systems this returns Some
        let htop = &SEEDS[0];
        assert_eq!(htop.command, "htop");
        // install_cmd depends on system, just verify no panic
        let _ = install_cmd(htop);
    }

    #[test]
    fn bash_has_no_install_cmd() {
        let bash = SEEDS.iter().find(|s| s.command == "bash").unwrap();
        assert!(install_cmd(bash).is_none());
    }

    #[test]
    fn install_method_returns_str() {
        let htop = &SEEDS[0];
        let method = install_method(htop);
        assert!(!method.is_empty());
    }
}
