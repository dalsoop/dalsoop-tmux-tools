use tmux_windowbar::config::template::Config;

use crate::form::{Field, Form};

/// A single settings key-value item.
#[derive(Debug, Clone)]
pub struct SettingItem {
    pub label: &'static str,
    pub value: String,
}

/// Build the flat list of all settings items from a config.
pub fn build_items(config: &Config) -> Vec<SettingItem> {
    let w = &config.window;
    let t = &config.theme;
    vec![
        SettingItem { label: "window.fg",              value: w.fg.clone() },
        SettingItem { label: "window.bg",              value: w.bg.clone() },
        SettingItem { label: "window.active_fg",       value: w.active_fg.clone() },
        SettingItem { label: "window.active_bg",       value: w.active_bg.clone() },
        SettingItem { label: "window.kill_fg",         value: w.kill_fg.clone() },
        SettingItem { label: "window.kill_bg",         value: w.kill_bg.clone() },
        SettingItem { label: "window.button_fg",       value: w.button_fg.clone() },
        SettingItem { label: "window.button_bg",       value: w.button_bg.clone() },
        SettingItem { label: "window.running_fg",      value: w.running_fg.clone() },
        SettingItem { label: "window.running_bg",      value: w.running_bg.clone() },
        SettingItem { label: "window.idle_fg",         value: w.idle_fg.clone() },
        SettingItem { label: "window.idle_bg",         value: w.idle_bg.clone() },
        SettingItem { label: "theme.users_label",      value: t.users_label.clone() },
        SettingItem { label: "theme.windows_label",    value: t.windows_label.clone() },
        SettingItem { label: "theme.panes_label",      value: t.panes_label.clone() },
        SettingItem { label: "theme.apps_label",       value: t.apps_label.clone() },
        SettingItem { label: "theme.user_viewed_fg",   value: t.user_viewed_fg.clone() },
        SettingItem { label: "theme.user_viewed_bg",   value: t.user_viewed_bg.clone() },
        SettingItem { label: "theme.user_session_fg",  value: t.user_session_fg.clone() },
        SettingItem { label: "theme.user_session_bg",  value: t.user_session_bg.clone() },
        SettingItem { label: "theme.ssh_connected_fg", value: t.ssh_connected_fg.clone() },
        SettingItem { label: "theme.ssh_connected_bg", value: t.ssh_connected_bg.clone() },
    ]
}

/// Build an edit form for the settings item at `idx`.
pub fn edit_form(items: &[SettingItem], idx: usize) -> Form {
    let item = &items[idx];
    Form::new(
        vec![Field { label: item.label, value: item.value.clone() }],
        Some(idx),
    )
}

/// Apply a completed single-field form back to the config.
pub fn apply_form(config: &mut Config, form: &Form) {
    let idx = match form.edit_idx {
        Some(i) => i,
        None => return,
    };
    let new_val = form.fields[0].value.clone();
    let w = &mut config.window;
    let t = &mut config.theme;
    match idx {
        0  => w.fg = new_val,
        1  => w.bg = new_val,
        2  => w.active_fg = new_val,
        3  => w.active_bg = new_val,
        4  => w.kill_fg = new_val,
        5  => w.kill_bg = new_val,
        6  => w.button_fg = new_val,
        7  => w.button_bg = new_val,
        8  => w.running_fg = new_val,
        9  => w.running_bg = new_val,
        10 => w.idle_fg = new_val,
        11 => w.idle_bg = new_val,
        12 => t.users_label = new_val,
        13 => t.windows_label = new_val,
        14 => t.panes_label = new_val,
        15 => t.apps_label = new_val,
        16 => t.user_viewed_fg = new_val,
        17 => t.user_viewed_bg = new_val,
        18 => t.user_session_fg = new_val,
        19 => t.user_session_bg = new_val,
        20 => t.ssh_connected_fg = new_val,
        21 => t.ssh_connected_bg = new_val,
        _  => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmux_windowbar::config::template::default_config;

    #[test]
    fn build_items_returns_22_entries() {
        let config = default_config();
        let items = build_items(&config);
        assert_eq!(items.len(), 22);
    }

    #[test]
    fn edit_form_single_field() {
        let config = default_config();
        let items = build_items(&config);
        let form = edit_form(&items, 0);
        assert_eq!(form.fields.len(), 1);
        assert_eq!(form.edit_idx, Some(0));
        assert_eq!(form.fields[0].value, config.window.fg);
    }

    #[test]
    fn apply_form_updates_window_fg() {
        let mut config = default_config();
        let items = build_items(&config);
        let mut form = edit_form(&items, 0);
        form.fields[0].value = "#123456".into();
        apply_form(&mut config, &form);
        assert_eq!(config.window.fg, "#123456");
    }

    #[test]
    fn apply_form_updates_theme_field() {
        let mut config = default_config();
        let items = build_items(&config);
        // index 12 = theme.users_label
        let mut form = edit_form(&items, 12);
        form.fields[0].value = "#aabbcc".into();
        apply_form(&mut config, &form);
        assert_eq!(config.theme.users_label, "#aabbcc");
    }
}
