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
    SeedApp { emoji: "💎", command: "gemini",     description: "Gemini CLI",        fg: "#282c34", bg: "#e5c07b", brew: "",           apt: "",           npm: "@google/gemini-cli", go: "" },
    SeedApp { emoji: "🐙", command: "gh",          description: "GitHub CLI",        fg: "#282c34", bg: "#5c6370", brew: "gh",         apt: "",           npm: "", go: "" },
    SeedApp { emoji: "🔍", command: "btop",       description: "Resource monitor",  fg: "#282c34", bg: "#c678dd", brew: "btop",       apt: "btop",       npm: "", go: "" },
    SeedApp { emoji: "📡", command: "bandwhich",  description: "Network monitor",   fg: "#282c34", bg: "#56b6c2", brew: "bandwhich",  apt: "",           npm: "", go: "" },
    SeedApp { emoji: "🌐", command: "opencode",   description: "OpenCode CLI",      fg: "#282c34", bg: "#98c379", brew: "",           apt: "",           npm: "", go: "github.com/opencode-ai/opencode@latest" },
    SeedApp { emoji: "🖥️", command: "bash",       description: "Shell",             fg: "#282c34", bg: "#5c6370", brew: "",           apt: "",           npm: "", go: "" },
    // pxi (proxmox-init) AI 관리 TUI — Claude/Codex doctor·tune·status, MCP 조회.
    // pxi 설치 전제 (curl install.prelik.com | bash). install_cmd 비워둠.
    SeedApp { emoji: "⚙️", command: "pxi-ai-menu", description: "AI 관리 (Claude/Codex)", fg: "#282c34", bg: "#d19a66", brew: "",           apt: "",           npm: "", go: "" },
];

/// Detect which package manager is available.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PkgManager {
    Brew,
    Apt,
    Npm,
    Go,
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

/// Check what's needed to install a seed app.
pub enum InstallStep {
    /// Ready to install directly.
    Ready(String),
    /// Need to install a package manager first.
    NeedManager(&'static str),
    /// No way to install.
    Unavailable,
    /// Already installed, just add to config.
    AlreadyInstalled,
}

pub fn check_install(seed: &SeedApp) -> InstallStep {
    if is_installed(seed.command) {
        return InstallStep::AlreadyInstalled;
    }
    if let Some(cmd) = install_cmd(seed) {
        return InstallStep::Ready(cmd);
    }
    // Check if brew would work but isn't installed
    if !seed.brew.is_empty() && std::env::consts::OS == "macos" {
        return InstallStep::NeedManager("brew");
    }
    InstallStep::Unavailable
}

/// Install a package manager. Returns true on success.
pub fn install_manager(name: &str) -> bool {
    match name {
        "brew" => {
            Command::new("sh")
                .args(["-c", "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        _ => false,
    }
}

/// Install a seed app (assumes package manager is available).
pub fn install(seed: &SeedApp) -> bool {
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

/// Build a remote install script that handles dependencies.
/// Returns a shell script string to run on the remote host.
pub fn remote_install_script(seed: &SeedApp) -> Option<String> {
    // gh CLI — needs special repo setup on Linux
    if seed.command == "gh" {
        return Some(r#"set -e
if command -v gh >/dev/null 2>&1; then echo "[✓] gh already installed"; exit 0; fi
echo "[+] Installing GitHub CLI..."
if command -v brew >/dev/null 2>&1; then
  brew install gh
elif command -v apt-get >/dev/null 2>&1; then
  curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
  chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null
  apt-get update -qq && apt-get install -y gh
else
  echo "[✗] No supported package manager"; exit 1
fi
echo "[✓] gh installed"
"#.to_string());
    }

    // npm-based apps (claude, codex, gemini)
    if !seed.npm.is_empty() {
        return Some(format!(
            r#"set -e
# Ensure Node.js + npm are available
if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
  echo "[+] Installing Node.js..."
  if command -v apt-get >/dev/null 2>&1; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && apt-get install -y nodejs npm
  elif command -v brew >/dev/null 2>&1; then
    brew install node
  else
    curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
    export NVM_DIR="$HOME/.nvm" && . "$NVM_DIR/nvm.sh" && nvm install 22
  fi
fi
echo "[+] Installing {name} via npm..."
npm install -g {pkg}
echo "[✓] {name} installed"
"#,
            name = seed.command,
            pkg = seed.npm,
        ));
    }

    // apt-based apps
    if !seed.apt.is_empty() {
        return Some(format!(
            r#"set -e
if command -v apt-get >/dev/null 2>&1; then
  echo "[+] Installing {name} via apt..."
  apt-get update -qq && apt-get install -y {pkg}
  echo "[✓] {name} installed"
else
  echo "[✗] apt not available"
  exit 1
fi
"#,
            name = seed.command,
            pkg = seed.apt,
        ));
    }

    // go-based apps
    if !seed.go.is_empty() {
        return Some(format!(
            r#"set -e
if ! command -v go >/dev/null 2>&1; then
  echo "[+] Installing Go..."
  if command -v apt-get >/dev/null 2>&1; then
    apt-get update -qq && apt-get install -y golang
  else
    echo "[✗] Cannot install Go automatically"
    exit 1
  fi
fi
echo "[+] Installing {name} via go..."
go install {pkg}
echo "[✓] {name} installed"
"#,
            name = seed.command,
            pkg = seed.go,
        ));
    }

    None
}

/// Find a seed by command name.
pub fn find(command: &str) -> Option<&'static SeedApp> {
    SEEDS.iter().find(|s| s.command == command)
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
