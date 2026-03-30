use std::process::Command;

/// A seedable app with install info.
pub struct SeedApp {
    pub emoji: &'static str,
    pub command: &'static str,
    pub install_cmd: &'static str, // e.g. "brew install htop"
    pub description: &'static str,
    pub fg: &'static str,
    pub bg: &'static str,
}

/// All recommended apps that can be seeded.
pub const SEEDS: &[SeedApp] = &[
    SeedApp { emoji: "📊", command: "htop",        install_cmd: "brew install htop",        description: "Process viewer",        fg: "#282c34", bg: "#d19a66" },
    SeedApp { emoji: "📂", command: "lazygit",     install_cmd: "brew install lazygit",     description: "Git TUI",               fg: "#282c34", bg: "#e06c75" },
    SeedApp { emoji: "🐳", command: "lazydocker",  install_cmd: "brew install lazydocker",  description: "Docker TUI",            fg: "#282c34", bg: "#61afef" },
    SeedApp { emoji: "🤖", command: "claude",      install_cmd: "npm install -g @anthropic-ai/claude-code", description: "Claude Code",  fg: "#282c34", bg: "#61afef" },
    SeedApp { emoji: "🧠", command: "codex",       install_cmd: "npm install -g @openai/codex", description: "OpenAI Codex CLI", fg: "#282c34", bg: "#98c379" },
    SeedApp { emoji: "🔍", command: "btop",        install_cmd: "brew install btop",        description: "Resource monitor",      fg: "#282c34", bg: "#c678dd" },
    SeedApp { emoji: "📡", command: "bandwhich",   install_cmd: "brew install bandwhich",   description: "Network monitor",       fg: "#282c34", bg: "#56b6c2" },
    SeedApp { emoji: "🌐", command: "opencode",    install_cmd: "go install github.com/nicholasgasior/opencode@latest", description: "OpenCode CLI", fg: "#282c34", bg: "#98c379" },
    SeedApp { emoji: "🖥️", command: "bash",        install_cmd: "",                         description: "Shell",                 fg: "#282c34", bg: "#5c6370" },
];

/// Check if a command is installed on the system.
pub fn is_installed(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Install a seed app. Returns true on success.
pub fn install(seed: &SeedApp) -> bool {
    if seed.install_cmd.is_empty() {
        return true; // built-in, nothing to install
    }
    // Run install command via sh
    Command::new("sh")
        .args(["-c", seed.install_cmd])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Display string for seed list: emoji + name + status + description
pub fn display(seed: &SeedApp) -> String {
    let status = if is_installed(seed.command) { "✓" } else { "✗" };
    format!("{} {:<14} {} {}", seed.emoji, seed.command, status, seed.description)
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
    fn display_contains_command() {
        let s = display(&SEEDS[0]);
        assert!(s.contains(SEEDS[0].command));
    }
}
