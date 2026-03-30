use tmux_windowbar::config::template::Config;

use crate::form::{Field, Form};

/// A single settings key-value item with getter/setter.
#[derive(Debug, Clone)]
pub struct SettingItem {
    pub label: &'static str,
    pub value: String,
}

/// Settings definition: (label, getter, setter)
type Getter = fn(&Config) -> String;
type Setter = fn(&mut Config, String);

struct SettingDef {
    label: &'static str,
    get: Getter,
    set: Setter,
}

// All settings defined in one place — adding/removing/reordering here
// automatically updates the TUI, with no index math to maintain.
const DEFS: &[SettingDef] = &[
    SettingDef { label: "window.default_app_mode", get: |c| c.window.default_app_mode.clone(), set: |c, v| c.window.default_app_mode = v },
    SettingDef { label: "window.show_kill_button", get: |c| c.window.show_kill_button.to_string(), set: |c, v| c.window.show_kill_button = v == "true" },
    SettingDef { label: "window.show_new_button",  get: |c| c.window.show_new_button.to_string(),  set: |c, v| c.window.show_new_button = v == "true" },
    SettingDef { label: "window.fg",               get: |c| c.window.fg.clone(),              set: |c, v| c.window.fg = v },
    SettingDef { label: "window.bg",               get: |c| c.window.bg.clone(),              set: |c, v| c.window.bg = v },
    SettingDef { label: "window.active_fg",        get: |c| c.window.active_fg.clone(),       set: |c, v| c.window.active_fg = v },
    SettingDef { label: "window.active_bg",        get: |c| c.window.active_bg.clone(),       set: |c, v| c.window.active_bg = v },
    SettingDef { label: "window.kill_fg",           get: |c| c.window.kill_fg.clone(),          set: |c, v| c.window.kill_fg = v },
    SettingDef { label: "window.kill_bg",           get: |c| c.window.kill_bg.clone(),          set: |c, v| c.window.kill_bg = v },
    SettingDef { label: "window.button_fg",        get: |c| c.window.button_fg.clone(),       set: |c, v| c.window.button_fg = v },
    SettingDef { label: "window.button_bg",        get: |c| c.window.button_bg.clone(),       set: |c, v| c.window.button_bg = v },
    SettingDef { label: "window.running_fg",       get: |c| c.window.running_fg.clone(),      set: |c, v| c.window.running_fg = v },
    SettingDef { label: "window.running_bg",       get: |c| c.window.running_bg.clone(),      set: |c, v| c.window.running_bg = v },
    SettingDef { label: "window.idle_fg",          get: |c| c.window.idle_fg.clone(),         set: |c, v| c.window.idle_fg = v },
    SettingDef { label: "window.idle_bg",          get: |c| c.window.idle_bg.clone(),         set: |c, v| c.window.idle_bg = v },
    SettingDef { label: "theme.users_label",       get: |c| c.theme.users_label.clone(),      set: |c, v| c.theme.users_label = v },
    SettingDef { label: "theme.windows_label",     get: |c| c.theme.windows_label.clone(),    set: |c, v| c.theme.windows_label = v },
    SettingDef { label: "theme.panes_label",       get: |c| c.theme.panes_label.clone(),      set: |c, v| c.theme.panes_label = v },
    SettingDef { label: "theme.apps_label",        get: |c| c.theme.apps_label.clone(),       set: |c, v| c.theme.apps_label = v },
    SettingDef { label: "theme.user_viewed_fg",    get: |c| c.theme.user_viewed_fg.clone(),   set: |c, v| c.theme.user_viewed_fg = v },
    SettingDef { label: "theme.user_viewed_bg",    get: |c| c.theme.user_viewed_bg.clone(),   set: |c, v| c.theme.user_viewed_bg = v },
    SettingDef { label: "theme.user_session_fg",   get: |c| c.theme.user_session_fg.clone(),  set: |c, v| c.theme.user_session_fg = v },
    SettingDef { label: "theme.user_session_bg",   get: |c| c.theme.user_session_bg.clone(),  set: |c, v| c.theme.user_session_bg = v },
    SettingDef { label: "theme.ssh_connected_fg",  get: |c| c.theme.ssh_connected_fg.clone(), set: |c, v| c.theme.ssh_connected_fg = v },
    SettingDef { label: "theme.ssh_connected_bg",  get: |c| c.theme.ssh_connected_bg.clone(), set: |c, v| c.theme.ssh_connected_bg = v },
];

pub fn build_items(config: &Config) -> Vec<SettingItem> {
    DEFS.iter()
        .map(|d| SettingItem { label: d.label, value: (d.get)(config) })
        .collect()
}

pub fn edit_form(items: &[SettingItem], idx: usize) -> Form {
    let item = &items[idx];
    Form::new(
        vec![Field { label: item.label, value: item.value.clone() }],
        Some(idx),
    )
}

pub fn apply_form(config: &mut Config, form: &Form) {
    let idx = match form.edit_idx {
        Some(i) => i,
        None => return,
    };
    if idx >= DEFS.len() { return; }
    let new_val = form.fields[0].value.clone();
    (DEFS[idx].set)(config, new_val);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmux_windowbar::config::template::default_config;

    #[test]
    fn build_items_count_matches_defs() {
        let config = default_config();
        let items = build_items(&config);
        assert_eq!(items.len(), DEFS.len());
    }

    #[test]
    fn edit_form_single_field() {
        let config = default_config();
        let items = build_items(&config);
        let form = edit_form(&items, 0);
        assert_eq!(form.fields.len(), 1);
        assert_eq!(form.edit_idx, Some(0));
    }

    #[test]
    fn apply_form_updates_value() {
        let mut config = default_config();
        let items = build_items(&config);
        let mut form = edit_form(&items, 0);
        form.fields[0].value = "pane".into();
        apply_form(&mut config, &form);
        assert_eq!(config.window.default_app_mode, "pane");
    }

    #[test]
    fn apply_form_updates_theme() {
        let mut config = default_config();
        let items = build_items(&config);
        // Find theme.users_label
        let idx = items.iter().position(|i| i.label == "theme.users_label").unwrap();
        let mut form = edit_form(&items, idx);
        form.fields[0].value = "#ff0000".into();
        apply_form(&mut config, &form);
        assert_eq!(config.theme.users_label, "#ff0000");
    }

    #[test]
    fn apply_form_bool_setting() {
        let mut config = default_config();
        let items = build_items(&config);
        let idx = items.iter().position(|i| i.label == "window.show_kill_button").unwrap();
        let mut form = edit_form(&items, idx);
        form.fields[0].value = "false".into();
        apply_form(&mut config, &form);
        assert!(!config.window.show_kill_button);
    }

    #[test]
    fn roundtrip_all_settings() {
        let config = default_config();
        let items = build_items(&config);
        for (i, item) in items.iter().enumerate() {
            let form = edit_form(&items, i);
            assert_eq!(form.fields[0].value, item.value, "mismatch at {}", item.label);
        }
    }
}
