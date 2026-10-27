//! HTML written from a Ghost document, for the markdown converter, which
//! cleans it with `ammonia` before anything else reads it.

/// HTML as it is written, with the kinds of content it left out.
#[derive(Debug, Default)]
pub(crate) struct Html {
    /// The HTML so far.
    pub(crate) text: String,
    /// The kinds of content that have no place in Khalid, such as a
    /// `signup` card, each once, in the order they first appear.
    pub(crate) left: Vec<String>,
}

impl Html {
    /// Adds markup as it is.
    pub(crate) fn tag(&mut self, markup: &str) {
        self.text.push_str(markup);
    }

    /// Adds `text` with its markup characters escaped, so a post about
    /// `<script>` shows the word.
    pub(crate) fn escaped(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '&' => self.text.push_str("&amp;"),
                '<' => self.text.push_str("&lt;"),
                '>' => self.text.push_str("&gt;"),
                '"' => self.text.push_str("&quot;"),
                c => self.text.push(c),
            }
        }
    }

    /// Notes that a kind of content was left out.
    pub(crate) fn leave(&mut self, kind: &str) {
        if !self.left.iter().any(|seen| seen == kind) {
            self.left.push(kind.to_owned());
        }
    }
}
