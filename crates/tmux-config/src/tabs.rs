use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Ssh,
    Apps,
    Proxmox,
    Dal,
    Settings,
}

impl Tab {
    pub fn titles() -> Vec<&'static str> {
        vec!["SSH", "Apps", "Proxmox", "Settings"]
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Ssh      => 0,
            Tab::Apps     => 1,
            Tab::Proxmox  => 2,
            Tab::Settings => 3,
            // Dal 은 UI 에서 숨김 — 순환/선택 대상에서 제외 (enum variant 는
            // 다른 모듈이 참조하고 있어 유지).
            Tab::Dal      => usize::MAX,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            1 => Tab::Apps,
            2 => Tab::Proxmox,
            3 => Tab::Settings,
            _ => Tab::Ssh,
        }
    }
}

pub fn render_tab_bar(f: &mut Frame, tab: Tab, area: Rect) {
    let titles: Vec<Line> = Tab::titles()
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::Rgb(171, 178, 191)))))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::Rgb(62, 68, 82))))
        .select(tab.index())
        .style(Style::default().fg(Color::Rgb(171, 178, 191)).bg(Color::Rgb(40, 44, 52)))
        .highlight_style(
            Style::default()
                .fg(Color::Rgb(97, 175, 239))
                .bg(Color::Rgb(40, 44, 52))
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" | "));

    f.render_widget(tabs, area);
}
