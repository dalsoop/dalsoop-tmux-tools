use std::path::Path;
use std::process::Command;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== tmux-tools config sync ===\n");

    // Read /etc/passwd for login users
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

    // Ensure root configs exist as source
    let root_sb = "/root/.config/tmux-sessionbar";
    let root_wb = "/root/.config/tmux-windowbar";
    if !Path::new(root_sb).exists() || !Path::new(root_wb).exists() {
        return Err("run `tmux-sessionbar init` and `tmux-windowbar init` as root first".into());
    }

    for user in &users {
        let home = format!("/home/{user}");
        if !Path::new(&home).exists() {
            println!("{user:<20} home not found, skip");
            continue;
        }

        // Sync sessionbar config
        let sb_dst = format!("{home}/.config/tmux-sessionbar");
        std::fs::create_dir_all(&sb_dst).ok();
        let sb_ok = Command::new("rsync")
            .args(["-a", &format!("{root_sb}/"), &format!("{sb_dst}/")])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        // Sync windowbar config
        let wb_dst = format!("{home}/.config/tmux-windowbar");
        std::fs::create_dir_all(&wb_dst).ok();
        let wb_ok = Command::new("rsync")
            .args(["-a", &format!("{root_wb}/"), &format!("{wb_dst}/")])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        // Generate .tmux.conf as user
        let conf_ok = Command::new("sudo")
            .args(["-iu", user, "tmux-sessionbar", "apply"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Fix ownership
        Command::new("chown")
            .args(["-R", &format!("{user}:{user}"), &format!("{home}/.config/tmux-sessionbar")])
            .status().ok();
        Command::new("chown")
            .args(["-R", &format!("{user}:{user}"), &format!("{home}/.config/tmux-windowbar")])
            .status().ok();
        Command::new("chown")
            .args([&format!("{user}:{user}"), &format!("{home}/.tmux.conf")])
            .status().ok();

        let sb_mark = if sb_ok { "✓" } else { "✗" };
        let wb_mark = if wb_ok { "✓" } else { "✗" };
        let conf_mark = if conf_ok { "✓" } else { "✗" };

        println!("{user:<20} sessionbar:{sb_mark}  windowbar:{wb_mark}  tmux.conf:{conf_mark}");
    }

    println!("\n=== sync complete ===");
    Ok(())
}
