use ratatui::widgets::ListState;

/// Wraps ratatui ListState with helpers for a list of items.
pub struct ListView {
    pub state: ListState,
    pub len: usize,
}

impl ListView {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(None);
        Self { state, len: 0 }
    }

    /// Sync the item count and ensure selection is within bounds.
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        if len == 0 {
            self.state.select(None);
        } else if let Some(sel) = self.state.selected() {
            if sel >= len {
                self.state.select(Some(len.saturating_sub(1)));
            }
        } else {
            // Auto-select first item when list becomes non-empty.
            self.state.select(Some(0));
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn select(&mut self, idx: usize) {
        if self.len > 0 && idx < self.len {
            self.state.select(Some(idx));
        }
    }

    pub fn move_down(&mut self) {
        if self.len == 0 {
            return;
        }
        let next = match self.state.selected() {
            Some(i) => (i + 1) % self.len,
            None => 0,
        };
        self.state.select(Some(next));
    }

    pub fn move_up(&mut self) {
        if self.len == 0 {
            return;
        }
        let prev = match self.state.selected() {
            Some(0) | None => self.len - 1,
            Some(i) => i - 1,
        };
        self.state.select(Some(prev));
    }
}

impl Default for ListView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_len_selects_first() {
        let mut lv = ListView::new();
        lv.set_len(3);
        assert_eq!(lv.selected(), Some(0));
    }

    #[test]
    fn set_len_zero_clears_selection() {
        let mut lv = ListView::new();
        lv.set_len(3);
        lv.set_len(0);
        assert_eq!(lv.selected(), None);
    }

    #[test]
    fn move_down_wraps() {
        let mut lv = ListView::new();
        lv.set_len(3);
        lv.move_down();
        assert_eq!(lv.selected(), Some(1));
        lv.move_down();
        lv.move_down();
        assert_eq!(lv.selected(), Some(0));
    }

    #[test]
    fn move_up_wraps() {
        let mut lv = ListView::new();
        lv.set_len(3);
        lv.move_up();
        assert_eq!(lv.selected(), Some(2));
    }

    #[test]
    fn clamps_on_shrink() {
        let mut lv = ListView::new();
        lv.set_len(5);
        lv.select(4);
        lv.set_len(3);
        assert_eq!(lv.selected(), Some(2));
    }
}
