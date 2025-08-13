#[derive(Default, Clone)]
pub(crate) struct NumericInput {
    pub value: String, // text buffer
    pub cursor: usize, // optional if you want editing in the middle
}

impl NumericInput {
    pub fn insert(&mut self, ch: char) {
        // Only allow digits and a single dot
        if ch.is_ascii_digit()
            || (ch == '.' && !self.value.contains('.'))
            || (ch == '-' && self.cursor == 0 && !self.value.contains('-'))
        {
            self.value.insert(self.cursor, ch);
            self.cursor += 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.value.parse().ok()
    }
}

impl From<String> for NumericInput {
    fn from(value: String) -> Self {
        let len = value.len();
        NumericInput { value, cursor: len }
    }
}
