use anyhow::Result;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use tmux_windowbar::config::template::{SshEntry, load_config};

use crate::save_and_apply;

pub fn manage() -> Result<()> {
    loop {
        let config = load_config()?;

        let mut items: Vec<String> = config.ssh.iter().map(|s| {
            let user = s.user.as_deref().unwrap_or("");
            let target = if user.is_empty() {
                s.host.clone()
            } else {
                format!("{user}@{}", s.host)
            };
            format!("{} {} ({})", s.emoji, s.name, target)
        }).collect();
        items.push("+ Add new".into());
        items.push("<- Back".into());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("SSH Hosts")
            .items(&items)
            .default(0)
            .interact()?;

        let ssh_count = config.ssh.len();
        if selection == ssh_count {
            add()?;
        } else if selection == ssh_count + 1 {
            break;
        } else {
            delete(selection)?;
        }
    }
    Ok(())
}

fn add() -> Result<()> {
    let theme = ColorfulTheme::default();
    let name: String = Input::with_theme(&theme).with_prompt("Name").interact_text()?;
    let host: String = Input::with_theme(&theme).with_prompt("Host").interact_text()?;
    let user: String = Input::with_theme(&theme)
        .with_prompt("User (optional)")
        .default(String::new())
        .interact_text()?;
    let emoji: String = Input::with_theme(&theme)
        .with_prompt("Emoji")
        .default("\u{1f5a5}\u{fe0f}".into())
        .interact_text()?;

    let mut config = load_config()?;
    config.ssh.push(SshEntry {
        name,
        host,
        user: if user.is_empty() { None } else { Some(user) },
        emoji,
        fg: "#abb2bf".into(),
        bg: "#3e4452".into(),
    });
    save_and_apply(&config)?;
    println!("Added");
    Ok(())
}

fn delete(idx: usize) -> Result<()> {
    let mut config = load_config()?;
    let name = config.ssh[idx].name.clone();

    let confirm = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Delete '{name}'?"))
        .default(false)
        .interact()?;

    if confirm {
        config.ssh.remove(idx);
        save_and_apply(&config)?;
        println!("Deleted '{name}'");
    }
    Ok(())
}
