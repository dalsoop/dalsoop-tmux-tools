use tmux_windowbar::config::template::{Config, SshEntry};

use crate::form::{Field, Form};

/// Build form fields for adding a new SSH entry.
pub fn add_form() -> Form {
    Form::new(
        vec![
            Field { label: "Name", value: String::new() },
            Field { label: "Host", value: String::new() },
            Field { label: "User", value: String::new() },
            Field { label: "Emoji", value: "\u{1f5a5}\u{fe0f}".into() },
            Field { label: "Type (ssh/proxmox)", value: "ssh".into() },
        ],
        None,
    )
}

/// Build form fields pre-filled from an existing entry.
pub fn edit_form(config: &Config, idx: usize) -> Form {
    let e = &config.ssh[idx];
    Form::new(
        vec![
            Field { label: "Name", value: e.name.clone() },
            Field { label: "Host", value: e.host.clone() },
            Field { label: "User", value: e.user.clone().unwrap_or_default() },
            Field { label: "Emoji", value: e.emoji.clone() },
            Field { label: "Type (ssh/proxmox)", value: e.r#type.clone() },
        ],
        Some(idx),
    )
}

/// Apply a completed form to the config.
pub fn apply_form(config: &mut Config, form: &Form) {
    let values = form.values();
    let name = values[0].to_owned();
    let host = values[1].to_owned();
    let user_str = values[2].to_owned();
    let emoji = values[3].to_owned();
    let type_str = values[4].to_owned();
    let user = if user_str.is_empty() { None } else { Some(user_str) };
    let entry_type = if type_str == "proxmox" { "proxmox".into() } else { "ssh".into() };

    match form.edit_idx {
        None => {
            config.ssh.push(SshEntry {
                name,
                host,
                user,
                emoji,
                fg: "#abb2bf".into(),
                bg: "#3e4452".into(),
                r#type: entry_type,
            });
        }
        Some(idx) => {
            let e = &mut config.ssh[idx];
            e.name = name;
            e.host = host;
            e.user = user;
            e.emoji = emoji;
            e.r#type = entry_type;
        }
    }
}

/// Format a single SSH entry for list display.
pub fn display(e: &SshEntry) -> String {
    let user = e.user.as_deref().unwrap_or("");
    let target = if user.is_empty() {
        e.host.clone()
    } else {
        format!("{user}@{}", e.host)
    };
    let type_tag = if e.r#type == "proxmox" { " [proxmox]" } else { "" };
    format!("{} {}  {}{}", e.emoji, e.name, target, type_tag)
}

/// Delete entry at idx.
pub fn delete(config: &mut Config, idx: usize) {
    if idx < config.ssh.len() {
        config.ssh.remove(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmux_windowbar::config::template::default_config;

    #[test]
    fn add_form_has_four_fields() {
        let f = add_form();
        assert_eq!(f.fields.len(), 5);
        assert_eq!(f.edit_idx, None);
    }

    #[test]
    fn edit_form_prefills_values() {
        let mut config = default_config();
        config.ssh.push(SshEntry {
            name: "proxmox".into(),
            host: "192.168.2.50".into(),
            user: Some("root".into()),
            emoji: "\u{1f5a5}\u{fe0f}".into(),
            fg: "#abb2bf".into(),
            bg: "#3e4452".into(),
            r#type: "ssh".into(),
        });
        let f = edit_form(&config, 0);
        assert_eq!(f.fields[0].value, "proxmox");
        assert_eq!(f.fields[1].value, "192.168.2.50");
        assert_eq!(f.fields[2].value, "root");
        assert_eq!(f.edit_idx, Some(0));
    }

    #[test]
    fn apply_form_adds_entry() {
        let mut config = default_config();
        let initial_count = config.ssh.len();
        let mut form = add_form();
        form.fields[0].value = "myhost".into();
        form.fields[1].value = "10.0.0.1".into();
        form.fields[2].value = "admin".into();
        form.fields[3].value = "\u{1f4bb}".into();
        apply_form(&mut config, &form);
        assert_eq!(config.ssh.len(), initial_count + 1);
        let added = config.ssh.last().unwrap();
        assert_eq!(added.name, "myhost");
        assert_eq!(added.user, Some("admin".into()));
    }

    #[test]
    fn apply_form_edits_entry() {
        let mut config = default_config();
        config.ssh.push(SshEntry {
            name: "old".into(),
            host: "1.1.1.1".into(),
            user: None,
            emoji: "\u{1f5a5}\u{fe0f}".into(),
            fg: "#abb2bf".into(),
            bg: "#3e4452".into(),
            r#type: "ssh".into(),
        });
        let mut form = edit_form(&config, 0);
        form.fields[0].value = "new".into();
        apply_form(&mut config, &form);
        assert_eq!(config.ssh[0].name, "new");
    }

    #[test]
    fn delete_removes_entry() {
        let mut config = default_config();
        config.ssh.push(SshEntry {
            name: "to-del".into(),
            host: "1.1.1.1".into(),
            user: None,
            emoji: "\u{1f5a5}\u{fe0f}".into(),
            fg: "#abb2bf".into(),
            bg: "#3e4452".into(),
            r#type: "ssh".into(),
        });
        assert_eq!(config.ssh.len(), 1);
        delete(&mut config, 0);
        assert_eq!(config.ssh.len(), 0);
    }

    #[test]
    fn display_formats_correctly() {
        let e = SshEntry {
            name: "proxmox".into(),
            host: "192.168.2.50".into(),
            user: Some("root".into()),
            emoji: "\u{1f5a5}\u{fe0f}".into(),
            fg: "#abb2bf".into(),
            bg: "#3e4452".into(),
            r#type: "ssh".into(),
        };
        let s = display(&e);
        assert!(s.contains("proxmox"));
        assert!(s.contains("root@192.168.2.50"));
    }
}
