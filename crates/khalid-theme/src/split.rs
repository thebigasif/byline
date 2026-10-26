//! Splitting CSS text at separators that sit outside strings and brackets.

/// Splits `text` wherever `is_separator` matches outside quotes, brackets
/// and parentheses, trims each piece and drops the empty ones.
pub(crate) fn split_top(text: &str, is_separator: impl Fn(char) -> bool) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut scan = Scanner::default();
    let mut start = 0;
    for (at, c) in text.char_indices() {
        let depth = scan.depth;
        if scan.step(c) && depth == 0 && is_separator(c) {
            pieces.extend(text.get(start..at));
            start = at.saturating_add(c.len_utf8());
        }
    }
    pieces.extend(text.get(start..));
    pieces
        .into_iter()
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .collect()
}

/// Tracks quotes, escapes and bracket depth one character at a time.
#[derive(Debug, Default)]
pub(crate) struct Scanner {
    /// The quote character of the string being read, if any.
    pub(crate) quote: Option<char>,
    /// Whether the last character was a backslash.
    pub(crate) escaped: bool,
    /// How many brackets, braces and parentheses are open.
    pub(crate) depth: usize,
}

impl Scanner {
    /// Takes `c` and says whether it is structural: outside a string and
    /// not escaped. Brackets change the depth after the call.
    pub(crate) fn step(&mut self, c: char) -> bool {
        if self.escaped {
            self.escaped = false;
            return false;
        }
        if c == '\\' {
            self.escaped = true;
            return false;
        }
        if let Some(quote) = self.quote {
            if c == quote {
                self.quote = None;
            }
            return false;
        }
        match c {
            '"' | '\'' => {
                self.quote = Some(c);
                return false;
            }
            '{' | '(' | '[' => self.depth = self.depth.saturating_add(1),
            '}' | ')' | ']' => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::split_top;

    #[test]
    fn splits_lists_only_at_the_top_level() {
        let pieces = split_top(
            "Inter, \"Segoe UI, Bold\", var(--a, b), 'it\\'s', sans-serif",
            |c| c == ',',
        );

        assert_eq!(
            pieces,
            [
                "Inter",
                "\"Segoe UI, Bold\"",
                "var(--a, b)",
                "'it\\'s'",
                "sans-serif"
            ]
        );
    }

    #[test]
    fn empty_pieces_are_dropped() {
        assert_eq!(split_top(" , a,, b ,", |c| c == ','), ["a", "b"]);
        assert_eq!(split_top("", |c| c == ','), Vec::<&str>::new());
    }
}
