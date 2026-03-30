/// A single field in the input form.
#[derive(Debug, Clone)]
pub struct Field {
    pub label: &'static str,
    pub value: String,
}

/// Active input form state.
#[derive(Debug, Clone)]
pub struct Form {
    pub fields: Vec<Field>,
    pub focused: usize,
    /// Index into the data list being edited; None means "add new".
    pub edit_idx: Option<usize>,
}

impl Form {
    pub fn new(fields: Vec<Field>, edit_idx: Option<usize>) -> Self {
        Self { fields, focused: 0, edit_idx }
    }

    /// Move focus to next field.
    pub fn next_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        self.focused = (self.focused + 1) % self.fields.len();
    }

    /// Move focus to previous field.
    pub fn prev_field(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        if self.focused == 0 {
            self.focused = self.fields.len() - 1;
        } else {
            self.focused -= 1;
        }
    }

    /// Handle a character input for the currently focused field.
    pub fn handle_char(&mut self, c: char) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.value.push(c);
        }
    }

    /// Handle backspace for the currently focused field.
    pub fn handle_backspace(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focused) {
            field.value.pop();
        }
    }

    /// Return values in order.
    pub fn values(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.value.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_form() -> Form {
        Form::new(
            vec![
                Field { label: "Name", value: "foo".into() },
                Field { label: "Host", value: "bar".into() },
            ],
            None,
        )
    }

    #[test]
    fn next_field_wraps() {
        let mut f = make_form();
        assert_eq!(f.focused, 0);
        f.next_field();
        assert_eq!(f.focused, 1);
        f.next_field();
        assert_eq!(f.focused, 0);
    }

    #[test]
    fn prev_field_wraps() {
        let mut f = make_form();
        f.prev_field();
        assert_eq!(f.focused, 1);
    }

    #[test]
    fn handle_char_appends_to_focused() {
        let mut f = make_form();
        f.handle_char('x');
        assert_eq!(f.fields[0].value, "foox");
    }

    #[test]
    fn handle_backspace_removes_last() {
        let mut f = make_form();
        f.handle_backspace();
        assert_eq!(f.fields[0].value, "fo");
    }

    #[test]
    fn values_returns_all() {
        let f = make_form();
        assert_eq!(f.values(), vec!["foo", "bar"]);
    }
}
