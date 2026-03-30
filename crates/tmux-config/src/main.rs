mod apps;
mod config_io;
mod form;
mod list_view;
mod settings;
mod ssh;
mod tabs;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use std::io;
use tmux_windowbar::config::template::Config;

use config_io::{load_config, save_and_apply};
use form::Form;
use list_view::ListView;
use settings::SettingItem;
use tabs::Tab;

// ─── palette ─────────────────────────────────────────────────────────────────
const BG:     Color = Color::Rgb(40,  44,  52);
const FG:     Color = Color::Rgb(171, 178, 191);
const GREEN:  Color = Color::Rgb(152, 195, 121);
const BLUE:   Color = Color::Rgb(97,  175, 239);
const RED:    Color = Color::Rgb(224, 108, 117);
const SUBTLE: Color = Color::Rgb(92,  99,  112);
const BORDER: Color = Color::Rgb(62,  68,  82);

// ─── mode ────────────────────────────────────────────────────────────────────
enum Mode {
    Normal,
    Editing,
    Confirming { label: String, idx: usize },
}

// ─── app ─────────────────────────────────────────────────────────────────────
struct App {
    tab:      Tab,
    ssh:      ListView,
    apps:     ListView,
    settings: ListView,
    mode:     Mode,
    form:     Option<Form>,
    config:   Config,
    /// Cached flat settings items (rebuilt when settings tab is entered).
    setting_items: Vec<SettingItem>,
    status_msg: Option<String>,
}

impl App {
    fn new(config: Config) -> Self {
        let setting_items = settings::build_items(&config);
        let mut app = Self {
            tab: Tab::Ssh,
            ssh: ListView::new(),
            apps: ListView::new(),
            settings: ListView::new(),
            mode: Mode::Normal,
            form: None,
            config,
            setting_items,
            status_msg: None,
        };
        app.sync_lengths();
        app
    }

    fn sync_lengths(&mut self) {
        self.ssh.set_len(self.config.ssh.len());
        self.apps.set_len(self.config.apps.len());
        self.settings.set_len(self.setting_items.len());
    }

    fn reload_settings(&mut self) {
        self.setting_items = settings::build_items(&self.config);
        self.settings.set_len(self.setting_items.len());
    }

    fn current_list_mut(&mut self) -> &mut ListView {
        match self.tab {
            Tab::Ssh      => &mut self.ssh,
            Tab::Apps     => &mut self.apps,
            Tab::Settings => &mut self.settings,
        }
    }

    fn move_down(&mut self) { self.current_list_mut().move_down(); }
    fn move_up(&mut self)   { self.current_list_mut().move_up(); }

    fn start_add(&mut self) {
        self.form = Some(match self.tab {
            Tab::Ssh      => ssh::add_form(),
            Tab::Apps     => apps::add_form(),
            Tab::Settings => return, // settings has no "add"
        });
        self.mode = Mode::Editing;
        self.status_msg = None;
    }

    fn start_edit(&mut self) {
        let form = match self.tab {
            Tab::Ssh => {
                let idx = match self.ssh.selected() { Some(i) => i, None => return };
                ssh::edit_form(&self.config, idx)
            }
            Tab::Apps => {
                let idx = match self.apps.selected() { Some(i) => i, None => return };
                apps::edit_form(&self.config, idx)
            }
            Tab::Settings => {
                let idx = match self.settings.selected() { Some(i) => i, None => return };
                settings::edit_form(&self.setting_items, idx)
            }
        };
        self.form = Some(form);
        self.mode = Mode::Editing;
        self.status_msg = None;
    }

    fn start_delete(&mut self) {
        match self.tab {
            Tab::Ssh => {
                let idx = match self.ssh.selected() { Some(i) => i, None => return };
                let label = self.config.ssh[idx].name.clone();
                self.mode = Mode::Confirming { label, idx };
            }
            Tab::Apps => {
                let idx = match self.apps.selected() { Some(i) => i, None => return };
                let label = self.config.apps[idx].command.clone();
                self.mode = Mode::Confirming { label, idx };
            }
            Tab::Settings => {} // no delete for settings
        }
        self.status_msg = None;
    }

    fn confirm_delete(&mut self) {
        let (idx,) = match &self.mode {
            Mode::Confirming { idx, .. } => (*idx,),
            _ => return,
        };
        match self.tab {
            Tab::Ssh  => ssh::delete(&mut self.config, idx),
            Tab::Apps => apps::delete(&mut self.config, idx),
            Tab::Settings => {}
        }
        let _ = save_and_apply(&self.config);
        self.sync_lengths();
        self.mode = Mode::Normal;
        self.status_msg = Some("Deleted.".into());
    }

    fn cancel(&mut self) {
        self.form = None;
        self.mode = Mode::Normal;
    }

    fn submit_form(&mut self) {
        let form = match self.form.take() {
            Some(f) => f,
            None => return,
        };
        match self.tab {
            Tab::Ssh  => ssh::apply_form(&mut self.config, &form),
            Tab::Apps => apps::apply_form(&mut self.config, &form),
            Tab::Settings => {
                settings::apply_form(&mut self.config, &form);
                self.reload_settings();
            }
        }
        let _ = save_and_apply(&self.config);
        self.sync_lengths();
        self.mode = Mode::Normal;
        self.status_msg = Some("Saved.".into());
    }

    fn handle_form_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let form = match &mut self.form {
            Some(f) => f,
            None => return,
        };
        match code {
            KeyCode::Char(c) => form.handle_char(c),
            KeyCode::Backspace => form.handle_backspace(),
            KeyCode::Tab => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    form.prev_field();
                } else {
                    form.next_field();
                }
            }
            KeyCode::BackTab => form.prev_field(),
            KeyCode::Enter => {
                // Drop the borrow so we can call submit_form
                let _ = form;
                self.submit_form();
            }
            KeyCode::Esc => self.cancel(),
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Tab | KeyCode::Char('\t') => {
                let next = (self.tab.index() + 1) % 3;
                self.tab = Tab::from_index(next);
                self.status_msg = None;
            }
            KeyCode::Char('1') => { self.tab = Tab::Ssh;      self.status_msg = None; }
            KeyCode::Char('2') => { self.tab = Tab::Apps;     self.status_msg = None; }
            KeyCode::Char('3') => { self.tab = Tab::Settings; self.status_msg = None; }
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up   | KeyCode::Char('k') => self.move_up(),
            KeyCode::Char('a') => self.start_add(),
            KeyCode::Char('e') | KeyCode::Enter => self.start_edit(),
            KeyCode::Char('d') => self.start_delete(),
            _ => {}
        }
        false
    }

    fn handle_confirm_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('y') | KeyCode::Enter => self.confirm_delete(),
            KeyCode::Char('n') | KeyCode::Esc   => { self.mode = Mode::Normal; }
            KeyCode::Char('q') => return true,
            _ => {}
        }
        false
    }
}

// ─── rendering ───────────────────────────────────────────────────────────────

fn render(f: &mut ratatui::Frame, app: &mut App) {
    let full = f.area();

    // Outer border block
    let outer = Block::default()
        .title(Span::styled(" tmux-config ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));
    let inner = outer.inner(full);
    f.render_widget(outer, full);

    // Split: tab bar (3) | body (fill) | hint bar (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // Tab bar
    tabs::render_tab_bar(f, app.tab, chunks[0]);

    // Body area (may be split for form overlay)
    render_body(f, app, chunks[1]);

    // Hint / status bar
    render_hint(f, app, chunks[2]);
}

fn render_body(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    match &app.mode {
        Mode::Normal => render_list(f, app, area),
        Mode::Confirming { label, .. } => {
            let label = label.clone();
            render_list(f, app, area);
            render_confirm_overlay(f, &label, area);
        }
        Mode::Editing => {
            // Split body: list (upper) | form (lower, ~10 lines)
            let form_height = {
                let fields = app.form.as_ref().map_or(0, |f| f.fields.len());
                // label + each field + divider + hint
                (fields as u16 + 4).min(area.height.saturating_sub(3))
            };
            let list_height = area.height.saturating_sub(form_height);
            let parts = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(list_height), Constraint::Length(form_height)])
                .split(area);
            render_list(f, app, parts[0]);
            render_form(f, app, parts[1]);
        }
    }
}

fn render_list(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let (title, items, list_view) = match app.tab {
        Tab::Ssh => {
            let items: Vec<ListItem> = app.config.ssh.iter()
                .map(|e| ListItem::new(ssh::display(e)))
                .collect();
            ("SSH Hosts", items, &mut app.ssh)
        }
        Tab::Apps => {
            let items: Vec<ListItem> = app.config.apps.iter()
                .map(|a| ListItem::new(apps::display(a)))
                .collect();
            ("Apps", items, &mut app.apps)
        }
        Tab::Settings => {
            let items: Vec<ListItem> = app.setting_items.iter()
                .map(|s| ListItem::new(format!("{:30}  {}", s.label, s.value)))
                .collect();
            ("Settings", items, &mut app.settings)
        }
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(format!(" {title} "), Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().fg(FG).bg(BG))
        .highlight_style(
            Style::default()
                .fg(BLUE)
                .bg(Color::Rgb(58, 63, 76))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut list_view.state);
}

fn render_form(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let form = match &app.form {
        Some(fm) => fm,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        let cursor = if i == form.focused { "\u{2588}" } else { "" };
        let label_style = if i == form.focused {
            Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SUBTLE)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:20} ", field.label), label_style),
            Span::styled(field.value.clone(), Style::default().fg(FG)),
            Span::styled(cursor, Style::default().fg(BLUE)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [Enter]", Style::default().fg(GREEN)),
        Span::raw(" Save  "),
        Span::styled("[Esc]", Style::default().fg(RED)),
        Span::raw(" Cancel  "),
        Span::styled("[Tab]", Style::default().fg(SUBTLE)),
        Span::raw(" Next field"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" Edit ", Style::default().fg(BLUE)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BLUE))
                .style(Style::default().bg(BG)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn render_confirm_overlay(f: &mut ratatui::Frame, label: &str, area: Rect) {
    // Center a small box in the area
    let w = 50u16.min(area.width.saturating_sub(4));
    let h = 5u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect::new(x, y, w, h);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Delete '"),
            Span::styled(label, Style::default().fg(RED)),
            Span::raw("' ?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y]", Style::default().fg(RED)),
            Span::raw(" Yes  "),
            Span::styled("[n/Esc]", Style::default().fg(GREEN)),
            Span::raw(" No"),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(" Confirm Delete ", Style::default().fg(RED)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(RED))
            .style(Style::default().bg(BG)),
    );
    f.render_widget(para, rect);
}

fn render_hint(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let text = if let Some(msg) = &app.status_msg {
        Line::from(Span::styled(msg.clone(), Style::default().fg(GREEN)))
    } else {
        match app.mode {
            Mode::Normal => Line::from(vec![
                Span::styled("[a]", Style::default().fg(BLUE)), Span::raw("dd  "),
                Span::styled("[e]", Style::default().fg(BLUE)), Span::raw("dit  "),
                Span::styled("[d]", Style::default().fg(RED)),  Span::raw("elete  "),
                Span::styled("[Tab]", Style::default().fg(SUBTLE)), Span::raw(" switch tab  "),
                Span::styled("[q]", Style::default().fg(SUBTLE)), Span::raw("uit"),
            ]),
            Mode::Editing => Line::from(vec![
                Span::styled("[Tab]", Style::default().fg(BLUE)), Span::raw(" next field  "),
                Span::styled("[Enter]", Style::default().fg(GREEN)), Span::raw(" save  "),
                Span::styled("[Esc]", Style::default().fg(RED)), Span::raw(" cancel"),
            ]),
            Mode::Confirming { .. } => Line::from(vec![
                Span::styled("[y]", Style::default().fg(RED)), Span::raw(" confirm  "),
                Span::styled("[n/Esc]", Style::default().fg(GREEN)), Span::raw(" cancel"),
            ]),
        }
    };
    let para = Paragraph::new(text)
        .style(Style::default().fg(FG).bg(BG));
    f.render_widget(para, area);
}

// ─── entry point ─────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = load_config()?;
    let mut app = App::new(config);

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| render(f, app))?;

        if let Event::Key(key) = event::read()? {
            let should_quit = match app.mode {
                Mode::Normal => app.handle_normal_key(key.code),
                Mode::Editing => {
                    app.handle_form_key(key.code, key.modifiers);
                    false
                }
                Mode::Confirming { .. } => app.handle_confirm_key(key.code),
            };
            if should_quit {
                break;
            }
        }
    }
    Ok(())
}
