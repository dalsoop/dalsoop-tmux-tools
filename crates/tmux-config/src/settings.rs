use anyhow::Result;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use tmux_windowbar::config::template::load_config;

use crate::save_and_apply;

pub fn manage() -> Result<()> {
    loop {
        let items = &["Window Colors", "Theme Colors", "Back"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Settings")
            .items(items)
            .default(0)
            .interact()?;

        match selection {
            0 => edit_window()?,
            1 => edit_theme()?,
            _ => break,
        }
    }
    Ok(())
}

fn edit_window() -> Result<()> {
    let mut config = load_config()?;
    let w = &config.window;
    let theme = ColorfulTheme::default();

    let fields: Vec<(&str, String)> = vec![
        ("FG", w.fg.clone()),
        ("BG", w.bg.clone()),
        ("Active FG", w.active_fg.clone()),
        ("Active BG", w.active_bg.clone()),
        ("Kill FG", w.kill_fg.clone()),
        ("Kill BG", w.kill_bg.clone()),
        ("Button FG", w.button_fg.clone()),
        ("Button BG", w.button_bg.clone()),
        ("Running FG", w.running_fg.clone()),
        ("Running BG", w.running_bg.clone()),
        ("Idle FG", w.idle_fg.clone()),
        ("Idle BG", w.idle_bg.clone()),
    ];

    // Show current values and let user pick which to edit
    let mut items: Vec<String> = fields
        .iter()
        .map(|(name, val)| format!("{name}: {val}"))
        .collect();
    items.push("Back".into());

    let selection = Select::with_theme(&theme)
        .with_prompt("Window Colors (select to edit)")
        .items(&items)
        .default(0)
        .interact()?;

    if selection >= fields.len() {
        return Ok(());
    }

    let (name, current) = &fields[selection];
    let new_val: String = Input::with_theme(&theme)
        .with_prompt(*name)
        .default(current.clone())
        .interact_text()?;

    match selection {
        0 => config.window.fg = new_val,
        1 => config.window.bg = new_val,
        2 => config.window.active_fg = new_val,
        3 => config.window.active_bg = new_val,
        4 => config.window.kill_fg = new_val,
        5 => config.window.kill_bg = new_val,
        6 => config.window.button_fg = new_val,
        7 => config.window.button_bg = new_val,
        8 => config.window.running_fg = new_val,
        9 => config.window.running_bg = new_val,
        10 => config.window.idle_fg = new_val,
        11 => config.window.idle_bg = new_val,
        _ => {}
    }

    save_and_apply(&config)?;
    println!("Updated");
    Ok(())
}

fn edit_theme() -> Result<()> {
    let mut config = load_config()?;
    let t = &config.theme;
    let theme = ColorfulTheme::default();

    let fields: Vec<(&str, String)> = vec![
        ("Users label", t.users_label.clone()),
        ("Windows label", t.windows_label.clone()),
        ("Panes label", t.panes_label.clone()),
        ("Apps label", t.apps_label.clone()),
        ("User viewed FG", t.user_viewed_fg.clone()),
        ("User viewed BG", t.user_viewed_bg.clone()),
        ("User session FG", t.user_session_fg.clone()),
        ("User session BG", t.user_session_bg.clone()),
        ("SSH connected FG", t.ssh_connected_fg.clone()),
        ("SSH connected BG", t.ssh_connected_bg.clone()),
    ];

    let mut items: Vec<String> = fields
        .iter()
        .map(|(name, val)| format!("{name}: {val}"))
        .collect();
    items.push("Back".into());

    let selection = Select::with_theme(&theme)
        .with_prompt("Theme Colors (select to edit)")
        .items(&items)
        .default(0)
        .interact()?;

    if selection >= fields.len() {
        return Ok(());
    }

    let (name, current) = &fields[selection];
    let new_val: String = Input::with_theme(&theme)
        .with_prompt(*name)
        .default(current.clone())
        .interact_text()?;

    match selection {
        0 => config.theme.users_label = new_val,
        1 => config.theme.windows_label = new_val,
        2 => config.theme.panes_label = new_val,
        3 => config.theme.apps_label = new_val,
        4 => config.theme.user_viewed_fg = new_val,
        5 => config.theme.user_viewed_bg = new_val,
        6 => config.theme.user_session_fg = new_val,
        7 => config.theme.user_session_bg = new_val,
        8 => config.theme.ssh_connected_fg = new_val,
        9 => config.theme.ssh_connected_bg = new_val,
        _ => {}
    }

    save_and_apply(&config)?;
    println!("Updated");
    Ok(())
}
