use crate::ai_status;
use crate::config::template;
use crate::config::tmux_conf;
use anyhow::{Context, Result, bail};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use tmux_fmt::shims;
use tmux_fmt::tmux;

const CLEAR_HISTORY_MARKER: &str = "tmux-clear-history";

fn clear_history_script() -> PathBuf {
    template::bin_dir().join(CLEAR_HISTORY_MARKER)
}

pub fn run() -> Result<()> {
    let home = tmux::home_dir();
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
    shims::install_shims(
        &template::bin_dir(),
        &binary_path,
        &shims::resolve_executable("tmux-windowbar")?,
    )?;
    ai_status::install(&template::bin_dir())?;

    let conf_content = tmux_conf::generate(&config, &binary_path);
    fs::write(&tmux_conf_path, &conf_content)?;
    println!("generated: {}", tmux_conf_path.display());

    let reload = Command::new("tmux")
        .args(["source-file", &tmux_conf_path.to_string_lossy()])
        .status();

    match reload {
        Ok(s) if s.success() => {
            println!("tmux config reloaded");
            let _ = Command::new("tmux-windowbar").args(["apply"]).status();
        }
        _ => println!("tmux not running — config will apply on next start"),
    }

    setup_maintenance(&config)?;

    Ok(())
}

fn setup_maintenance(config: &template::Config) -> Result<()> {
    let script_path = clear_history_script();
    if config.maintenance.auto_clear {
        let script = "#!/bin/bash\n\
            tmux list-panes -a -F '#{session_name}:#{window_index}.#{pane_index}' 2>/dev/null | while read pane; do\n\
            \ttmux clear-history -t \"$pane\" 2>/dev/null\n\
            done\n";

        if let Some(parent) = script_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(&script_path)?;
        file.write_all(script.as_bytes())?;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
        println!("installed: {}", script_path.display());

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
            config.maintenance.clear_interval,
            script_path.display()
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
            config.maintenance.clear_interval,
            script_path.display()
        );
    } else {
        remove_cron_entry()?;

        if script_path.exists() {
            fs::remove_file(&script_path)?;
            println!("removed: {}", script_path.display());
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
