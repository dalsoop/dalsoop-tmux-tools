use anyhow::Result;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use tmux_windowbar::config::template::{AppEntry, load_config};

use crate::save_and_apply;

pub fn manage() -> Result<()> {
    loop {
        let config = load_config()?;

        let mut items: Vec<String> = config.apps.iter().map(|a| {
            format!("{} {} [{}]", a.emoji, a.command, a.mode)
        }).collect();
        items.push("+ Add new".into());
        items.push("<- Back".into());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Apps")
            .items(&items)
            .default(0)
            .interact()?;

        let apps_count = config.apps.len();
        if selection == apps_count {
            add()?;
        } else if selection == apps_count + 1 {
            break;
        } else {
            delete(selection)?;
        }
    }
    Ok(())
}

fn add() -> Result<()> {
    let theme = ColorfulTheme::default();
    let emoji: String = Input::with_theme(&theme)
        .with_prompt("Emoji")
        .interact_text()?;
    let command: String = Input::with_theme(&theme)
        .with_prompt("Command")
        .interact_text()?;

    let mode_items = &["window", "pane"];
    let mode_idx = Select::with_theme(&theme)
        .with_prompt("Mode")
        .items(mode_items)
        .default(0)
        .interact()?;
    let mode = mode_items[mode_idx].to_string();

    let fg: String = Input::with_theme(&theme)
        .with_prompt("Foreground color (hex)")
        .default("#282c34".into())
        .interact_text()?;
    let bg: String = Input::with_theme(&theme)
        .with_prompt("Background color (hex)")
        .default("#61afef".into())
        .interact_text()?;

    let mut config = load_config()?;
    config.apps.push(AppEntry {
        emoji,
        command,
        mode,
        fg,
        bg,
    });
    save_and_apply(&config)?;
    println!("Added");
    Ok(())
}

fn delete(idx: usize) -> Result<()> {
    let mut config = load_config()?;
    let command = config.apps[idx].command.clone();

    let confirm = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Delete '{command}'?"))
        .default(false)
        .interact()?;

    if confirm {
        config.apps.remove(idx);
        save_and_apply(&config)?;
        println!("Deleted '{command}'");
    }
    Ok(())
}
