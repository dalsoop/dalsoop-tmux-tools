//! Type-safe builder for tmux status bar format strings.
//!
//! Prevents common bugs: missing `#[norange]`, forgotten `#[default]` resets,
//! mismatched style directives.
//!
//! # Examples
//!
//! ```
//! use tmux_fmt::{Block, Line};
//!
//! // Clickable session block with auto range/norange
//! let block = Block::click("main")
//!     .style("#282c34", "#98c379").bold()
//!     .text(" main ")
//!     .build();
//! assert_eq!(block, "#[range=user|main]#[fg=#282c34,bg=#98c379,bold] main #[norange default]");
//!
//! // Label with auto reset
//! let label = Block::label("Sessions", "#98c379").build();
//! assert_eq!(label, "#[fg=#98c379,bold]Sessions #[default]");
//!
//! // Compose a full line
//! let line = Line::new()
//!     .left()
//!     .push(&label)
//!     .push(&block)
//!     .right()
//!     .push("#[fg=#abb2bf] stats ")
//!     .build();
//! ```

pub mod tmux;

use std::fmt;

// ── Constants ──

/// Reset all styles to default.
pub const RESET: &str = "#[default]";

// ── Style tag helper ──

/// Build a tmux style tag from components.
///
/// ```
/// use tmux_fmt::style_tag;
/// assert_eq!(style_tag(Some("#fff"), Some("#000"), true), "#[fg=#fff,bg=#000,bold]");
/// assert_eq!(style_tag(Some("#fff"), None, false), "#[fg=#fff]");
/// assert_eq!(style_tag(None, None, false), "");
/// ```
pub fn style_tag(fg: Option<&str>, bg: Option<&str>, bold: bool) -> String {
    let mut attrs = Vec::new();
    if let Some(fg) = fg {
        attrs.push(format!("fg={fg}"));
    }
    if let Some(bg) = bg {
        attrs.push(format!("bg={bg}"));
    }
    if bold {
        attrs.push("bold".to_string());
    }
    if attrs.is_empty() {
        String::new()
    } else {
        format!("#[{}]", attrs.join(","))
    }
}

// ── Block builder ──

/// Builder for a single tmux format block.
///
/// Four creation modes:
/// - `Block::click(id)` — clickable range block, auto-wraps with `range`/`norange default`
/// - `Block::label(text, fg)` — bold label, auto-appends `#[default]`
/// - `Block::plain()` — raw styled block, no auto-reset
/// - `Block::tmux_conf(fg, bg)` — for `.tmux.conf` window-status-format (text may contain `#I`, `#W`)
pub struct Block {
    kind: BlockKind,
    fg: Option<String>,
    bg: Option<String>,
    bold: bool,
    text: String,
}

enum BlockKind {
    /// Clickable block: wraps in `#[range=user|id]...#[norange default]`
    Click(String),
    /// Label: `#[fg=X,bold]text #[default]`
    Label,
    /// Plain styled block: `#[fg=X,bg=Y]text` — no auto-reset
    Plain,
    /// For .tmux.conf: `#[fg=X,bg=Y] text ` — wrapped in quotes for tmux set command
    TmuxConf,
}

impl Block {
    /// Clickable block with `range=user|id`.
    /// Automatically closes with `#[norange default]`.
    pub fn click(id: &str) -> Self {
        Self {
            kind: BlockKind::Click(id.to_string()),
            fg: None,
            bg: None,
            bold: false,
            text: String::new(),
        }
    }

    /// Bold label: `#[fg=color,bold]text #[default]`
    pub fn label(text: &str, fg: &str) -> Self {
        Self {
            kind: BlockKind::Label,
            fg: Some(fg.to_string()),
            bg: None,
            bold: true,
            text: text.to_string(),
        }
    }

    /// Plain styled block with no auto-reset.
    pub fn plain() -> Self {
        Self {
            kind: BlockKind::Plain,
            fg: None,
            bg: None,
            bold: false,
            text: String::new(),
        }
    }

    /// For `.tmux.conf` window-status-format strings.
    /// Produces `#[fg=X,bg=Y,bold] text ` — text can contain tmux variables like `#I`, `#W`.
    pub fn tmux_conf(fg: &str, bg: &str) -> Self {
        Self {
            kind: BlockKind::TmuxConf,
            fg: Some(fg.to_string()),
            bg: Some(bg.to_string()),
            bold: false,
            text: String::new(),
        }
    }

    /// Set foreground and background colors.
    pub fn style(mut self, fg: &str, bg: &str) -> Self {
        self.fg = Some(fg.to_string());
        self.bg = Some(bg.to_string());
        self
    }

    /// Set foreground color only.
    pub fn fg(mut self, fg: &str) -> Self {
        self.fg = Some(fg.to_string());
        self
    }

    /// Set background color only.
    pub fn bg(mut self, bg: &str) -> Self {
        self.bg = Some(bg.to_string());
        self
    }

    /// Enable bold.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Set the display text.
    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    /// Build the format string.
    pub fn build(&self) -> String {
        let tag = style_tag(
            self.fg.as_deref(),
            self.bg.as_deref(),
            self.bold,
        );
        let mut out = String::new();

        match &self.kind {
            BlockKind::Click(id) => {
                out.push_str(&format!("#[range=user|{id}]"));
                out.push_str(&tag);
                out.push_str(&self.text);
                out.push_str("#[norange default]");
            }
            BlockKind::Label => {
                out.push_str(&tag);
                out.push_str(&self.text);
                out.push(' ');
                out.push_str(RESET);
            }
            BlockKind::Plain => {
                out.push_str(&tag);
                out.push_str(&self.text);
            }
            BlockKind::TmuxConf => {
                out.push_str(&tag);
                out.push_str(&self.text);
            }
        }

        out
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.build())
    }
}

// ── Convenience functions ──

/// Shorthand: clickable block with style and text.
///
/// ```
/// use tmux_fmt::click;
/// let s = click("main", "#282c34", "#98c379", true, " main ");
/// assert_eq!(s, "#[range=user|main]#[fg=#282c34,bg=#98c379,bold] main #[norange default]");
/// ```
pub fn click(id: &str, fg: &str, bg: &str, bold: bool, text: &str) -> String {
    let mut b = Block::click(id).style(fg, bg).text(text);
    if bold {
        b = b.bold();
    }
    b.build()
}

/// Shorthand: bold label.
///
/// ```
/// use tmux_fmt::label;
/// let s = label("Sessions", "#98c379");
/// assert_eq!(s, "#[fg=#98c379,bold]Sessions #[default]");
/// ```
pub fn label(text: &str, fg: &str) -> String {
    Block::label(text, fg).build()
}

/// Shorthand: plain styled text (no range, no reset).
///
/// ```
/// use tmux_fmt::styled;
/// let s = styled("#abb2bf", "#3e4452", " 5.2 ");
/// assert_eq!(s, "#[fg=#abb2bf,bg=#3e4452] 5.2 ");
/// ```
pub fn styled(fg: &str, bg: &str, text: &str) -> String {
    Block::plain().style(fg, bg).text(text).build()
}

/// Shorthand: plain styled text with bold.
///
/// ```
/// use tmux_fmt::styled_bold;
/// let s = styled_bold("#282c34", "#98c379", " main ");
/// assert_eq!(s, "#[fg=#282c34,bg=#98c379,bold] main ");
/// ```
pub fn styled_bold(fg: &str, bg: &str, text: &str) -> String {
    Block::plain().style(fg, bg).bold().text(text).build()
}

/// Shorthand: `.tmux.conf` window-status-format string.
///
/// ```
/// use tmux_fmt::conf_style;
/// let s = conf_style("#aaa", "#333", false, " #I:#W ");
/// assert_eq!(s, "#[fg=#aaa,bg=#333] #I:#W ");
/// let s = conf_style("#000", "#fff", true, " #I:#W ");
/// assert_eq!(s, "#[fg=#000,bg=#fff,bold] #I:#W ");
/// ```
pub fn conf_style(fg: &str, bg: &str, bold: bool, text: &str) -> String {
    let mut b = Block::tmux_conf(fg, bg).text(text);
    if bold {
        b = b.bold();
    }
    b.build()
}

// ── Line builder ──

/// Builder for a complete tmux status-format line.
///
/// Handles `#[align=left default]` and `#[align=right default]` sections.
pub struct Line {
    sections: Vec<Section>,
}

struct Section {
    align: Align,
    parts: Vec<String>,
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

impl Line {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    /// Start or switch to a left-aligned section.
    pub fn left(mut self) -> Self {
        self.sections.push(Section {
            align: Align::Left,
            parts: Vec::new(),
        });
        self
    }

    /// Start or switch to a right-aligned section.
    pub fn right(mut self) -> Self {
        self.sections.push(Section {
            align: Align::Right,
            parts: Vec::new(),
        });
        self
    }

    /// Add a pre-built `Block` to the current section.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, block: &Block) -> Self {
        if let Some(section) = self.sections.last_mut() {
            section.parts.push(block.build());
        }
        self
    }

    /// Add any string (pre-built block output, raw format, or literal text)
    /// to the current section.
    pub fn push(mut self, s: &str) -> Self {
        if let Some(section) = self.sections.last_mut() {
            section.parts.push(s.to_string());
        }
        self
    }

    /// Build the complete format line.
    pub fn build(&self) -> String {
        let mut out = String::new();
        for section in &self.sections {
            let align = match section.align {
                Align::Left => "left",
                Align::Right => "right",
            };
            out.push_str(&format!("#[align={align} default]"));
            for part in &section.parts {
                out.push_str(part);
            }
        }
        out
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.build())
    }
}

// ── Fallback format ──

/// Tmux built-in window list format for when tmux-windowbar is unavailable.
/// Uses tmux's native `#{W:...}` template syntax.
///
/// Parameters:
/// - `left_content`: content before the window list (e.g. session label + blocks)
/// - `right_content`: content after the window list (e.g. stats + view switcher)
pub fn fallback_window_list(left_content: &str, right_content: &str) -> String {
    format!(
        "#[align=left default]{left_content}\
         #[list=on align=left]\
         #[list=left-marker]<#[list=right-marker]>\
         #[list=on]\
         #{{W:\
         #[range=window|#{{window_index}} #{{E:window-status-style}}]\
         #[push-default]#{{T:window-status-format}}#[pop-default]\
         #[norange default]#{{?window_end_flag,,#{{window-status-separator}}}},\
         #[range=window|#{{window_index}} list=focus #{{E:window-status-current-style}}]\
         #[push-default]#{{T:window-status-current-format}}#[pop-default]\
         #[norange list=on default]#{{?window_end_flag,,#{{window-status-separator}}}}\
         }}\
         #[nolist align=right default]{right_content}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_block_active() {
        let s = Block::click("main")
            .style("#282c34", "#98c379")
            .bold()
            .text(" main ")
            .build();
        assert_eq!(
            s,
            "#[range=user|main]#[fg=#282c34,bg=#98c379,bold] main #[norange default]"
        );
    }

    #[test]
    fn click_block_inactive() {
        let s = Block::click("2")
            .style("#abb2bf", "#3e4452")
            .text(" 2 ")
            .build();
        assert_eq!(
            s,
            "#[range=user|2]#[fg=#abb2bf,bg=#3e4452] 2 #[norange default]"
        );
    }

    #[test]
    fn kill_button() {
        let s = Block::click("_kmain")
            .fg("#e06c75")
            .bg("#282c34")
            .text(" x ")
            .build();
        assert_eq!(
            s,
            "#[range=user|_kmain]#[fg=#e06c75,bg=#282c34] x #[norange default]"
        );
    }

    #[test]
    fn label_block() {
        let s = Block::label("Sessions", "#98c379").build();
        assert_eq!(s, "#[fg=#98c379,bold]Sessions #[default]");
    }

    #[test]
    fn plain_block() {
        let s = Block::plain()
            .style("#abb2bf", "#3e4452")
            .text(" 5.2 ")
            .build();
        assert_eq!(s, "#[fg=#abb2bf,bg=#3e4452] 5.2 ");
    }

    #[test]
    fn shorthand_click() {
        let s = click("_app0", "#282c34", "#c678dd", false, " 🔐 spf ");
        assert_eq!(
            s,
            "#[range=user|_app0]#[fg=#282c34,bg=#c678dd] 🔐 spf #[norange default]"
        );
    }

    #[test]
    fn shorthand_label() {
        let s = label("Apps", "#e06c75");
        assert_eq!(s, "#[fg=#e06c75,bold]Apps #[default]");
    }

    #[test]
    fn shorthand_styled() {
        let s = styled("#abb2bf", "#3e4452", " 5.2 ");
        assert_eq!(s, "#[fg=#abb2bf,bg=#3e4452] 5.2 ");
    }

    #[test]
    fn shorthand_styled_bold() {
        let s = styled_bold("#282c34", "#98c379", " main ");
        assert_eq!(s, "#[fg=#282c34,bg=#98c379,bold] main ");
    }

    #[test]
    fn shorthand_conf_style() {
        let s = conf_style("#aaa", "#333", false, " #I:#W ");
        assert_eq!(s, "#[fg=#aaa,bg=#333] #I:#W ");
    }

    #[test]
    fn shorthand_conf_style_bold() {
        let s = conf_style("#000", "#fff", true, " #I:#W ");
        assert_eq!(s, "#[fg=#000,bg=#fff,bold] #I:#W ");
    }

    #[test]
    fn tmux_conf_block() {
        let s = Block::tmux_conf("#aaa", "#333")
            .text(" #I:#W ")
            .build();
        assert_eq!(s, "#[fg=#aaa,bg=#333] #I:#W ");
    }

    #[test]
    fn style_tag_full() {
        assert_eq!(
            style_tag(Some("#fff"), Some("#000"), true),
            "#[fg=#fff,bg=#000,bold]"
        );
    }

    #[test]
    fn style_tag_fg_only() {
        assert_eq!(style_tag(Some("#fff"), None, false), "#[fg=#fff]");
    }

    #[test]
    fn style_tag_empty() {
        assert_eq!(style_tag(None, None, false), "");
    }

    #[test]
    fn line_left_right() {
        let lbl = label("Sessions", "#98c379");
        let sess = click("main", "#282c34", "#98c379", true, " main ");
        let stats = styled("#abb2bf", "#3e4452", " 5.2 ");

        let line = Line::new()
            .left()
            .push(&lbl)
            .push(&sess)
            .right()
            .push(&stats)
            .build();

        assert!(line.starts_with("#[align=left default]"));
        assert!(line.contains("#[align=right default]"));
        assert!(line.contains("#[range=user|main]"));
    }

    #[test]
    fn line_left_only() {
        let lbl = label("Windows", "#c678dd");
        let line = Line::new().left().push(&lbl).build();
        assert_eq!(
            line,
            "#[align=left default]#[fg=#c678dd,bold]Windows #[default]"
        );
    }

    #[test]
    fn display_trait() {
        let block = Block::label("Test", "#ff0000");
        let s = format!("{block}");
        assert_eq!(s, "#[fg=#ff0000,bold]Test #[default]");
    }

    #[test]
    fn empty_style_no_tag() {
        let s = Block::plain().text("hello").build();
        assert_eq!(s, "hello");
    }

    #[test]
    fn click_guarantees_norange() {
        let s = Block::click("x").text(" x ").build();
        assert!(s.contains("#[range=user|x]"));
        assert!(s.contains("#[norange default]"));
    }

    #[test]
    fn fallback_contains_window_template() {
        let s = fallback_window_list("LEFT", "RIGHT");
        assert!(s.starts_with("#[align=left default]LEFT"));
        assert!(s.contains("#[list=on align=left]"));
        assert!(s.contains("#{W:"));
        assert!(s.ends_with("RIGHT"));
    }

    #[test]
    fn reset_constant() {
        assert_eq!(RESET, "#[default]");
    }

    // ── Domain invariant tests ──

    /// Every Block::click() output must have balanced #[range=...] and #[norange].
    /// Unbalanced range/norange = broken clickable regions in tmux status bar.
    #[test]
    fn domain_click_always_balanced_range_norange() {
        let test_ids = [
            "main",
            "2",
            "_kmain",
            "_app0",
            "",
            "session with spaces",
            "unicode-세션",
            "a-very-long-session-name-that-goes-on-and-on",
        ];

        for id in &test_ids {
            let output = Block::click(id)
                .style("#000", "#fff")
                .text(&format!(" {id} "))
                .build();

            let range_count = output.matches("#[range=").count();
            let norange_count = output.matches("#[norange").count();

            assert_eq!(
                range_count, 1,
                "expected exactly 1 #[range=...] for id={id:?}, got {range_count} in: {output}"
            );
            assert_eq!(
                norange_count, 1,
                "expected exactly 1 #[norange] for id={id:?}, got {norange_count} in: {output}"
            );

            // range must come before norange
            let range_pos = output.find("#[range=").unwrap();
            let norange_pos = output.find("#[norange").unwrap();
            assert!(
                range_pos < norange_pos,
                "range must precede norange for id={id:?}: {output}"
            );
        }
    }

    /// The click() shorthand must also produce balanced range/norange.
    #[test]
    fn domain_click_shorthand_balanced_range_norange() {
        let ids = ["main", "_k1", "_app0", ""];
        for id in &ids {
            let output = click(id, "#000", "#fff", false, " x ");
            assert!(
                output.contains("#[range=") && output.contains("#[norange"),
                "shorthand click missing range/norange for id={id:?}: {output}"
            );
        }
    }

    /// Label blocks must always end with #[default] reset.
    /// Missing reset = style bleeding into subsequent blocks.
    #[test]
    fn domain_label_always_resets() {
        let labels = ["Sessions", "Windows", "Apps", "", "한글"];
        for text in &labels {
            let output = Block::label(text, "#98c379").build();
            assert!(
                output.ends_with(RESET),
                "label({text:?}) missing terminal #[default]: {output}"
            );
        }
    }
}
