use crate::config::template;
use crate::config::tmux_conf;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let home = home_dir();
    let tmux_conf_path = home.join(".tmux.conf");
    let config_path = template::config_path();

    if !config_path.exists() {
        return Err(format!(
            "config not found: {}\nrun `tmux-sessionbar init` first.",
            config_path.display()
        )
        .into());
    }

    let config = template::load_config()?;
    let binary_path = std::env::current_exe()?
        .to_string_lossy()
        .to_string();

    let conf_content = tmux_conf::generate(&config, &binary_path);
    fs::write(&tmux_conf_path, &conf_content)?;
    println!("generated: {}", tmux_conf_path.display());

    let reload = Command::new("tmux")
        .args(["source-file", &tmux_conf_path.to_string_lossy()])
        .status();

    match reload {
        Ok(s) if s.success() => {
            println!("tmux config reloaded");

            // Re-apply windowbar settings (mouse click bindings, hooks)
            // These are runtime-only and not persisted in .tmux.conf
            let _ = Command::new("tmux-windowbar")
                .args(["apply"])
                .status();
        }
        _ => println!("tmux not running — config will apply on next start"),
    }

    Ok(())
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}
