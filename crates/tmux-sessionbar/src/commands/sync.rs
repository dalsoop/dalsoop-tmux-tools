use anyhow::{Result, bail};
use std::path::Path;
use std::process::Command;

pub fn run() -> Result<()> {
    println!("=== tmux-tools sync ===\n");

    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let mut users: Vec<String> = Vec::new();
    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 7 {
            continue;
        }
        let name = fields[0];
        let uid: u32 = fields[2].parse().unwrap_or(0);
        let shell = fields[6];
        if shell.contains("nologin") || shell.contains("/false") {
            continue;
        }
        if uid >= 1000 {
            users.push(name.to_string());
        }
    }

    if users.is_empty() {
        println!("no user accounts found to sync");
        return Ok(());
    }

    let root_sb = "/root/.config/tmux-sessionbar";
    let root_wb = "/root/.config/tmux-windowbar";
    if !Path::new(root_sb).exists() || !Path::new(root_wb).exists() {
        bail!("run `tmux-sessionbar init` as root first");
    }

    let root_tpm = "/root/.tmux/plugins/tpm";

    for user in &users {
        let home = format!("/home/{user}");
        if !Path::new(&home).exists() {
            println!("{user:<20} home not found, skip");
            continue;
        }

        let sb_dst = format!("{home}/.config/tmux-sessionbar");
        std::fs::create_dir_all(&sb_dst).ok();
        let sb_ok = Command::new("rsync")
            .args(["-a", &format!("{root_sb}/"), &format!("{sb_dst}/")])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let wb_dst = format!("{home}/.config/tmux-windowbar");
        std::fs::create_dir_all(&wb_dst).ok();
        let wb_ok = Command::new("rsync")
            .args(["-a", &format!("{root_wb}/"), &format!("{wb_dst}/")])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let user_tpm = format!("{home}/.tmux/plugins/tpm");
        if Path::new(root_tpm).exists() && !Path::new(&user_tpm).exists() {
            std::fs::create_dir_all(format!("{home}/.tmux/plugins")).ok();
            Command::new("cp")
                .args(["-r", root_tpm, &user_tpm])
                .status()
                .ok();
        }

        let root_plugins = "/root/.tmux/plugins";
        let user_plugins = format!("{home}/.tmux/plugins");
        if Path::new(root_plugins).exists() {
            Command::new("rsync")
                .args([
                    "-a",
                    "--exclude=tpm",
                    &format!("{root_plugins}/"),
                    &format!("{user_plugins}/"),
                ])
                .status()
                .ok();
        }

        let conf_ok = Command::new("sudo")
            .args(["-iu", user, "tmux-sessionbar", "apply"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        for dir in &[
            format!("{home}/.config/tmux-sessionbar"),
            format!("{home}/.config/tmux-windowbar"),
            format!("{home}/.tmux"),
        ] {
            Command::new("chown")
                .args(["-R", &format!("{user}:{user}"), dir])
                .status()
                .ok();
        }
        Command::new("chown")
            .args([&format!("{user}:{user}"), &format!("{home}/.tmux.conf")])
            .status()
            .ok();

        let sb_mark = if sb_ok { "✓" } else { "✗" };
        let wb_mark = if wb_ok { "✓" } else { "✗" };
        let conf_mark = if conf_ok { "✓" } else { "✗" };

        println!("{user:<20} config:{sb_mark}  windowbar:{wb_mark}  tmux.conf:{conf_mark}  tpm:✓");
    }

    println!("\n=== sync complete ===");
    Ok(())
}
