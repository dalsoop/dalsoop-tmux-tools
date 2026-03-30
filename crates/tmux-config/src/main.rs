mod apps;
mod ssh;

use anyhow::Result;
use dialoguer::{Select, theme::ColorfulTheme};
use tmux_windowbar::config::template::Config;

fn main() -> Result<()> {
    loop {
        let items = &["SSH Hosts", "Apps", "Exit"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("tmux-config")
            .items(items)
            .default(0)
            .interact()?;

        match selection {
            0 => ssh::manage()?,
            1 => apps::manage()?,
            2 => break,
            _ => break,
        }
    }
    Ok(())
}

pub(crate) fn save_and_apply(config: &Config) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(tmux_windowbar::config::template::config_path(), content)?;
    let _ = std::process::Command::new("tmux-windowbar")
        .args(["apply"])
        .status();
    let _ = std::process::Command::new("tmux-sessionbar")
        .args(["render-status", "left"])
        .status();
    Ok(())
}
