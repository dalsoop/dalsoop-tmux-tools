use crate::config::template::{self, default_config};
use crate::config::tmux_conf;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn run() -> Result<()> {
    let home = home_dir();
    let tmux_conf_path = home.join(".tmux.conf");
    let config_dir = template::config_dir();
    let config_path = template::config_path();

    println!("=== tmux-sessionbar init ===\n");

    // 0. Ensure tmux socket dir exists and binaries are in PATH
    ensure_tmux_tmpdir();
    ensure_in_path("tmux-sessionbar");
    ensure_in_path("tmux-windowbar");

    // 1. Backup existing .tmux.conf
    if tmux_conf_path.exists() {
        let backup = home.join(".tmux.conf.bak");
        fs::copy(&tmux_conf_path, &backup)?;
        println!("[1/7] backed up .tmux.conf -> .tmux.conf.bak");
    } else {
        println!("[1/7] no existing .tmux.conf");
    }

    // 2. Create sessionbar config
    fs::create_dir_all(&config_dir)?;
    if !config_path.exists() {
        let config = default_config();
        let content = toml::to_string_pretty(&config)?;
        fs::write(&config_path, &content)?;
        println!("[2/7] created sessionbar config: {}", config_path.display());
    } else {
        println!("[2/7] sessionbar config exists: {}", config_path.display());
    }

    // 3. Generate .tmux.conf
    let config = template::load_config()?;
    let binary_path = std::env::current_exe()
        .context("failed to get current exe path")?
        .to_string_lossy()
        .to_string();
    let conf_content = tmux_conf::generate(&config, &binary_path);
    fs::write(&tmux_conf_path, &conf_content)?;
    println!("[3/7] generated: {}", tmux_conf_path.display());

    // 4. Install TPM if missing
    let tpm_dir = home.join(".tmux/plugins/tpm");
    if !tpm_dir.exists() {
        println!("[4/7] installing TPM...");
        let output = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "https://github.com/tmux-plugins/tpm",
                &tpm_dir.to_string_lossy(),
            ])
            .output()
            .context("failed to run git clone for TPM")?;
        if output.status.success() {
            println!("[4/7] TPM installed");
        } else {
            eprintln!(
                "[4/7] TPM install failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        println!("[4/7] TPM already installed");
    }

    // 5. Init windowbar (config + bindings)
    let wb_config_dir = home.join(".config/tmux-windowbar");
    fs::create_dir_all(&wb_config_dir)?;
    let wb_config_path = wb_config_dir.join("config.toml");
    if !wb_config_path.exists() {
        let wb_result = Command::new("tmux-windowbar").args(["init"]).output();
        match wb_result {
            Ok(o) if o.status.success() => println!("[5/7] windowbar initialized"),
            _ => println!("[5/7] windowbar init skipped (binary not found or tmux not running)"),
        }
    } else {
        println!("[5/7] windowbar config exists");
    }

    // 6. Reload tmux + apply windowbar
    let reload = Command::new("tmux")
        .args(["source-file", &tmux_conf_path.to_string_lossy()])
        .status();

    match reload {
        Ok(s) if s.success() => {
            println!("[6/7] tmux config reloaded");
            let _ = Command::new("tmux-windowbar").args(["apply"]).status();
        }
        _ => println!("[6/7] tmux not running — will apply on next start"),
    }

    // 7. Install plugins via TPM
    let tpm_install = home.join(".tmux/plugins/tpm/bin/install_plugins");
    if tpm_install.exists() {
        println!("[7/7] installing plugins...");
        let output = Command::new(&tpm_install)
            .output()
            .context("failed to run TPM install_plugins")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut count = 0;
        for line in stdout.lines() {
            if line.contains("download success") {
                count += 1;
                println!("  {line}");
            }
        }
        if count == 0 {
            println!("[7/7] all plugins already installed");
        } else {
            println!("[7/7] {count} plugins installed");
        }
    } else {
        println!(
            "[7/7] skipped plugin install (TPM not ready, run `tmux-sessionbar plugin-install` later)"
        );
    }

    println!("\n=== done ===");
    println!("tmux-sessionbar init completed. All features active.");
    println!("edit {} to customize.", config_path.display());

    Ok(())
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/root".into()))
}

fn ensure_tmux_tmpdir() {
    let uid = Command::new("id")
        .args(["-u"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "0".into());
    let dir = format!("/tmp/tmux-{uid}");
    if !std::path::Path::new(&dir).exists() {
        let _ = fs::create_dir_all(&dir);
        let _ = Command::new("chmod").args(["700", &dir]).status();
    }
}

fn ensure_in_path(name: &str) {
    let local_bin = format!("/usr/local/bin/{name}");
    let usr_bin = format!("/usr/bin/{name}");

    if std::path::Path::new(&local_bin).exists() && !std::path::Path::new(&usr_bin).exists() {
        let path = std::env::var("PATH").unwrap_or_default();
        if !path.contains("/usr/local/bin") {
            let _ = std::os::unix::fs::symlink(&local_bin, &usr_bin);
        }
    }
}
