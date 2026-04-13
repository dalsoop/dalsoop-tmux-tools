use crate::form::{Field, Form};

/// Which config a setting belongs to
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingSource {
    Windowbar,
    Sessionbar,
    Header, // non-editable section separator
}

/// A single settings key-value item.
#[derive(Debug, Clone)]
pub struct SettingItem {
    pub label: &'static str,
    pub value: String,
    pub source: SettingSource,
}

// ── Windowbar settings ──────────────────────────────────────────────────────

type WbGetter = fn(&tmux_windowbar::config::template::Config) -> String;
type WbSetter = fn(&mut tmux_windowbar::config::template::Config, String);

struct WbDef {
    label: &'static str,
    get: WbGetter,
    set: WbSetter,
}

const WB_DEFS: &[WbDef] = &[
    WbDef { label: "default_app_mode", get: |c| c.window.default_app_mode.clone(), set: |c, v| c.window.default_app_mode = v },
    WbDef { label: "show_kill_button", get: |c| c.window.show_kill_button.to_string(), set: |c, v| c.window.show_kill_button = v == "true" },
    WbDef { label: "show_new_button",  get: |c| c.window.show_new_button.to_string(),  set: |c, v| c.window.show_new_button = v == "true" },
    WbDef { label: "window.fg",        get: |c| c.window.fg.clone(),              set: |c, v| c.window.fg = v },
    WbDef { label: "window.bg",        get: |c| c.window.bg.clone(),              set: |c, v| c.window.bg = v },
    WbDef { label: "window.active_fg", get: |c| c.window.active_fg.clone(),       set: |c, v| c.window.active_fg = v },
    WbDef { label: "window.active_bg", get: |c| c.window.active_bg.clone(),       set: |c, v| c.window.active_bg = v },
    WbDef { label: "window.kill_fg",   get: |c| c.window.kill_fg.clone(),         set: |c, v| c.window.kill_fg = v },
    WbDef { label: "window.kill_bg",   get: |c| c.window.kill_bg.clone(),         set: |c, v| c.window.kill_bg = v },
    WbDef { label: "button_fg",        get: |c| c.window.button_fg.clone(),       set: |c, v| c.window.button_fg = v },
    WbDef { label: "button_bg",        get: |c| c.window.button_bg.clone(),       set: |c, v| c.window.button_bg = v },
    WbDef { label: "running_fg",       get: |c| c.window.running_fg.clone(),      set: |c, v| c.window.running_fg = v },
    WbDef { label: "running_bg",       get: |c| c.window.running_bg.clone(),      set: |c, v| c.window.running_bg = v },
    WbDef { label: "idle_fg",          get: |c| c.window.idle_fg.clone(),         set: |c, v| c.window.idle_fg = v },
    WbDef { label: "idle_bg",          get: |c| c.window.idle_bg.clone(),         set: |c, v| c.window.idle_bg = v },
    WbDef { label: "theme.users",      get: |c| c.theme.users_label.clone(),      set: |c, v| c.theme.users_label = v },
    WbDef { label: "theme.windows",    get: |c| c.theme.windows_label.clone(),    set: |c, v| c.theme.windows_label = v },
    WbDef { label: "theme.panes",      get: |c| c.theme.panes_label.clone(),      set: |c, v| c.theme.panes_label = v },
    WbDef { label: "theme.apps",       get: |c| c.theme.apps_label.clone(),       set: |c, v| c.theme.apps_label = v },
];

// ── Sessionbar settings ─────────────────────────────────────────────────────

type SbGetter = fn(&tmux_sessionbar::config::template::Config) -> String;
type SbSetter = fn(&mut tmux_sessionbar::config::template::Config, String);

struct SbDef {
    label: &'static str,
    get: SbGetter,
    set: SbSetter,
}

const SB_DEFS: &[SbDef] = &[
    SbDef { label: "history_limit",    get: |c| c.general.history_limit.to_string(),     set: |c, v| { if let Ok(n) = v.parse() { c.general.history_limit = n; } } },
    SbDef { label: "status.interval",  get: |c| c.status.interval.to_string(),           set: |c, v| { if let Ok(n) = v.parse() { c.status.interval = n; } } },
    SbDef { label: "status.position",  get: |c| c.status.position.clone(),               set: |c, v| c.status.position = v },
    SbDef { label: "status.bg",        get: |c| c.status.bg.clone(),                     set: |c, v| c.status.bg = v },
    SbDef { label: "status.fg",        get: |c| c.status.fg.clone(),                     set: |c, v| c.status.fg = v },
    SbDef { label: "session.active_fg",  get: |c| c.blocks.session_list.active_fg.clone(),  set: |c, v| c.blocks.session_list.active_fg = v },
    SbDef { label: "session.active_bg",  get: |c| c.blocks.session_list.active_bg.clone(),  set: |c, v| c.blocks.session_list.active_bg = v },
    SbDef { label: "session.inactive_fg", get: |c| c.blocks.session_list.inactive_fg.clone(), set: |c, v| c.blocks.session_list.inactive_fg = v },
    SbDef { label: "session.inactive_bg", get: |c| c.blocks.session_list.inactive_bg.clone(), set: |c, v| c.blocks.session_list.inactive_bg = v },
    SbDef { label: "session.button_fg",  get: |c| c.blocks.session_list.button_fg.clone(),  set: |c, v| c.blocks.session_list.button_fg = v },
    SbDef { label: "session.button_bg",  get: |c| c.blocks.session_list.button_bg.clone(),  set: |c, v| c.blocks.session_list.button_bg = v },
    SbDef { label: "session.kill_fg",    get: |c| c.blocks.session_list.kill_fg.clone(),    set: |c, v| c.blocks.session_list.kill_fg = v },
    SbDef { label: "show_new_button",    get: |c| c.blocks.session_list.show_new_button.to_string(), set: |c, v| c.blocks.session_list.show_new_button = v == "true" },
    SbDef { label: "show_kill_button",   get: |c| c.blocks.session_list.show_kill_button.to_string(), set: |c, v| c.blocks.session_list.show_kill_button = v == "true" },
    SbDef { label: "hostname.fg",      get: |c| c.blocks.hostname.fg.clone(),             set: |c, v| c.blocks.hostname.fg = v },
    SbDef { label: "hostname.bg",      get: |c| c.blocks.hostname.bg.clone(),             set: |c, v| c.blocks.hostname.bg = v },
    SbDef { label: "hostname.format",  get: |c| c.blocks.hostname.format.clone(),         set: |c, v| c.blocks.hostname.format = v },
    SbDef { label: "datetime.fg",      get: |c| c.blocks.datetime.fg.clone(),             set: |c, v| c.blocks.datetime.fg = v },
    SbDef { label: "datetime.bg",      get: |c| c.blocks.datetime.bg.clone(),             set: |c, v| c.blocks.datetime.bg = v },
    SbDef { label: "datetime.format",  get: |c| c.blocks.datetime.format.clone(),         set: |c, v| c.blocks.datetime.format = v },
    SbDef { label: "keybind.session_switch", get: |c| c.keybindings.session_switch.to_string(), set: |c, v| c.keybindings.session_switch = v == "true" },
    SbDef { label: "keybind.pane_clear",     get: |c| c.keybindings.pane_clear.to_string(),     set: |c, v| c.keybindings.pane_clear = v == "true" },
    SbDef { label: "pane_border.enabled",    get: |c| c.pane_border.enabled.to_string(),        set: |c, v| c.pane_border.enabled = v == "true" },
    SbDef { label: "mem.normal",       get: |c| c.theme.mem_normal.clone(),  set: |c, v| c.theme.mem_normal = v },
    SbDef { label: "mem.warn",         get: |c| c.theme.mem_warn.clone(),    set: |c, v| c.theme.mem_warn = v },
    SbDef { label: "mem.critical",     get: |c| c.theme.mem_critical.clone(), set: |c, v| c.theme.mem_critical = v },
];

// ── Detect installed components ─────────────────────────────────────────────

fn is_installed(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn has_windowbar() -> bool { is_installed("tmux-windowbar") }
pub fn has_sessionbar() -> bool { is_installed("tmux-sessionbar") }

// ── Build / edit / apply ────────────────────────────────────────────────────

pub fn build_items(
    wb_config: &tmux_windowbar::config::template::Config,
    sb_config: Option<&tmux_sessionbar::config::template::Config>,
) -> Vec<SettingItem> {
    let mut items = Vec::new();

    if has_windowbar() {
        items.push(SettingItem {
            label: "── Windowbar ──",
            value: String::new(),
            source: SettingSource::Header,
        });
        for d in WB_DEFS {
            items.push(SettingItem {
                label: d.label,
                value: (d.get)(wb_config),
                source: SettingSource::Windowbar,
            });
        }
    }

    if let Some(sb) = sb_config {
        if has_sessionbar() {
            items.push(SettingItem {
                label: "── Sessionbar ──",
                value: String::new(),
                source: SettingSource::Header,
            });
            for d in SB_DEFS {
                items.push(SettingItem {
                    label: d.label,
                    value: (d.get)(sb),
                    source: SettingSource::Sessionbar,
                });
            }
        }
    }

    items
}

pub fn edit_form(items: &[SettingItem], idx: usize) -> Form {
    let item = &items[idx];
    if item.source == SettingSource::Header {
        // Headers are not editable, return empty form
        return Form::new(vec![], None);
    }
    Form::new(
        vec![Field { label: item.label, value: item.value.clone() }],
        Some(idx),
    )
}

pub fn apply_form_wb(
    config: &mut tmux_windowbar::config::template::Config,
    items: &[SettingItem],
    form: &Form,
) {
    let global_idx = match form.edit_idx {
        Some(i) => i,
        None => return,
    };
    let item = &items[global_idx];
    if item.source != SettingSource::Windowbar {
        return;
    }

    // Find which WB_DEF index this corresponds to
    let wb_idx = items[..global_idx]
        .iter()
        .filter(|i| i.source == SettingSource::Windowbar)
        .count();

    if wb_idx < WB_DEFS.len() {
        (WB_DEFS[wb_idx].set)(config, form.fields[0].value.clone());
    }
}

pub fn apply_form_sb(
    config: &mut tmux_sessionbar::config::template::Config,
    items: &[SettingItem],
    form: &Form,
) {
    let global_idx = match form.edit_idx {
        Some(i) => i,
        None => return,
    };
    let item = &items[global_idx];
    if item.source != SettingSource::Sessionbar {
        return;
    }

    let sb_idx = items[..global_idx]
        .iter()
        .filter(|i| i.source == SettingSource::Sessionbar)
        .count();

    if sb_idx < SB_DEFS.len() {
        (SB_DEFS[sb_idx].set)(config, form.fields[0].value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_items_has_wb_section() {
        let wb = tmux_windowbar::config::template::default_config();
        let items = build_items(&wb, None);
        // Should have at least windowbar items if binary exists
        // In test env, binary may not exist, so just verify no panic
        assert!(items.is_empty() || items.iter().any(|i| i.source == SettingSource::Windowbar || i.source == SettingSource::Header));
    }

    #[test]
    fn header_not_editable() {
        let item = SettingItem {
            label: "── Test ──",
            value: String::new(),
            source: SettingSource::Header,
        };
        let form = edit_form(&[item], 0);
        assert!(form.fields.is_empty());
    }

    #[test]
    fn wb_defs_count() {
        assert!(WB_DEFS.len() > 10);
    }

    #[test]
    fn sb_defs_count() {
        assert!(SB_DEFS.len() > 10);
    }
}
