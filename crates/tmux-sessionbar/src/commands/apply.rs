use crate::config::template;
use crate::config::tmux_conf;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

const CLEAR_HISTORY_SCRIPT: &str = "/usr/local/bin/tmux-clear-history";
const CLEAR_HISTORY_MARKER: &str = "tmux-clear-history";

pub fn run() -> Result<()> {
    let home = home_dir();
    let tmux_conf_path = home.join(".tmux.conf");
    let config_path = template::config_path();

    if !config_path.exists() {
        bail!(
            "config not found: {}\nrun `tmux-sessionbar init` first.",
            config_path.display()
        );
    }

    let config = template::load_config()?;

    // Backfill new fields (e.g. [theme]) into existing config
    let updated = toml::to_string_pretty(&config)?;
    fs::write(&config_path, &updated)?;

    let binary_path = std::env::current_exe()
        .context("failed to get current exe path")?
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
            let _ = Command::new("tmux-windowbar")
                .args(["apply"])
                .status();
        }
        _ => println!("tmux not running — config will apply on next start"),
    }

    setup_maintenance(&config)?;

    Ok(())
}

fn setup_maintenance(config: &template::Config) -> Result<()> {
    if config.maintenance.auto_clear {
        let script = "#!/bin/bash\n\
            tmux list-panes -a -F '#{session_name}:#{window_index}.#{pane_index}' 2>/dev/null | while read pane; do\n\
            \ttmux clear-history -t \"$pane\" 2>/dev/null\n\
            done\n";

        let mut file = fs::File::create(CLEAR_HISTORY_SCRIPT)?;
        file.write_all(script.as_bytes())?;
        fs::set_permissions(CLEAR_HISTORY_SCRIPT, fs::Permissions::from_mode(0o755))?;
        println!("installed: {}", CLEAR_HISTORY_SCRIPT);

        let existing = Command::new("crontab")
            .arg("-l")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let filtered: Vec<&str> = existing
            .lines()
            .filter(|line| !line.contains(CLEAR_HISTORY_MARKER))
            .collect();

        let cron_entry = format!(
            "*/{} * * * * {}",
            config.maintenance.clear_interval, CLEAR_HISTORY_SCRIPT
        );

        let mut new_crontab = filtered.join("\n");
        if !new_crontab.is_empty() && !new_crontab.ends_with('\n') {
            new_crontab.push('\n');
        }
        new_crontab.push_str(&cron_entry);
        new_crontab.push('\n');

        let mut child = Command::new("crontab")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("failed to run crontab")?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(new_crontab.as_bytes())?;
        }
        child.wait()?;

        println!(
            "cron installed: every {} min -> {}",
            config.maintenance.clear_interval, CLEAR_HISTORY_SCRIPT
        );
    } else {
        remove_cron_entry()?;

        if std::path::Path::new(CLEAR_HISTORY_SCRIPT).exists() {
            fs::remove_file(CLEAR_HISTORY_SCRIPT)?;
            println!("removed: {}", CLEAR_HISTORY_SCRIPT);
        }
    }

    Ok(())
}

fn remove_cron_entry() -> Result<()> {
    let existing = Command::new("crontab")
        .arg("-l")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if existing.contains(CLEAR_HISTORY_MARKER) {
        let filtered: Vec<&str> = existing
            .lines()
            .filter(|line| !line.contains(CLEAR_HISTORY_MARKER))
            .collect();

        let new_crontab = filtered.join("\n") + "\n";

        let mut child = Command::new("crontab")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("failed to run crontab")?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(new_crontab.as_bytes())?;
        }
        child.wait()?;
        println!("cron entry removed: {}", CLEAR_HISTORY_MARKER);
    }

    Ok(())
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}
