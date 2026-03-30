use tmux_windowbar::config::template::{AppEntry, Config};

use crate::form::{Field, Form};

/// Build form fields for adding a new app entry.
pub fn add_form(config: &Config) -> Form {
    Form::new(
        vec![
            Field { label: "Emoji", value: String::new() },
            Field { label: "Command", value: String::new() },
            Field { label: "Mode (window/pane)", value: config.window.default_app_mode.clone() },
            Field { label: "FG color", value: "#282c34".into() },
            Field { label: "BG color", value: "#61afef".into() },
        ],
        None,
    )
}

/// Build form fields pre-filled from an existing app entry.
pub fn edit_form(config: &Config, idx: usize) -> Form {
    let a = &config.apps[idx];
    Form::new(
        vec![
            Field { label: "Emoji", value: a.emoji.clone() },
            Field { label: "Command", value: a.command.clone() },
            Field { label: "Mode (window/pane)", value: a.mode.clone() },
            Field { label: "FG color", value: a.fg.clone() },
            Field { label: "BG color", value: a.bg.clone() },
        ],
        Some(idx),
    )
}

/// Apply a completed form to the config.
pub fn apply_form(config: &mut Config, form: &Form) {
    let values = form.values();
    let emoji = values[0].to_owned();
    let command = values[1].to_owned();
    let mode = {
        let raw = values[2].trim().to_lowercase();
        if raw == "pane" { "pane".into() } else { "window".into() }
    };
    let fg = values[3].to_owned();
    let bg = values[4].to_owned();

    match form.edit_idx {
        None => {
            config.apps.push(AppEntry { emoji, command, mode, fg, bg });
        }
        Some(idx) => {
            let a = &mut config.apps[idx];
            a.emoji = emoji;
            a.command = command;
            a.mode = mode;
            a.fg = fg;
            a.bg = bg;
        }
    }
}

/// Format a single app entry for list display.
pub fn display(a: &AppEntry) -> String {
    format!("{} {}  [{}]  fg={} bg={}", a.emoji, a.command, a.mode, a.fg, a.bg)
}

/// Delete entry at idx.
pub fn delete(config: &mut Config, idx: usize) {
    if idx < config.apps.len() {
        config.apps.remove(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmux_windowbar::config::template::default_config;

    #[test]
    fn add_form_has_five_fields() {
        let config = default_config(); let f = add_form(&config);
        assert_eq!(f.fields.len(), 5);
        assert_eq!(f.edit_idx, None);
    }

    #[test]
    fn edit_form_prefills_values() {
        let config = default_config();
        let f = edit_form(&config, 0);
        assert_eq!(f.fields[1].value, config.apps[0].command);
        assert_eq!(f.edit_idx, Some(0));
    }

    #[test]
    fn apply_form_adds_entry() {
        let mut config = default_config();
        let initial_count = config.apps.len();
        let mut form = add_form(&config);
        form.fields[0].value = "\u{1f4e6}".into();
        form.fields[1].value = "cargo".into();
        form.fields[2].value = "pane".into();
        form.fields[3].value = "#fff".into();
        form.fields[4].value = "#000".into();
        apply_form(&mut config, &form);
        assert_eq!(config.apps.len(), initial_count + 1);
        let added = config.apps.last().unwrap();
        assert_eq!(added.command, "cargo");
        assert_eq!(added.mode, "pane");
    }

    #[test]
    fn apply_form_edits_entry() {
        let mut config = default_config();
        let original_cmd = config.apps[0].command.clone();
        let mut form = edit_form(&config, 0);
        form.fields[1].value = "newcmd".into();
        apply_form(&mut config, &form);
        assert_ne!(config.apps[0].command, original_cmd);
        assert_eq!(config.apps[0].command, "newcmd");
    }

    #[test]
    fn apply_form_defaults_unknown_mode_to_window() {
        let mut config = default_config();
        let mut form = add_form(&config);
        form.fields[0].value = "\u{1f4e6}".into();
        form.fields[1].value = "foo".into();
        form.fields[2].value = "unknown".into();
        form.fields[3].value = "#fff".into();
        form.fields[4].value = "#000".into();
        apply_form(&mut config, &form);
        assert_eq!(config.apps.last().unwrap().mode, "window");
    }

    #[test]
    fn delete_removes_entry() {
        let mut config = default_config();
        let initial = config.apps.len();
        delete(&mut config, 0);
        assert_eq!(config.apps.len(), initial - 1);
    }
}
