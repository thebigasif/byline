//! Writing the front matter of a content file, so that `khalid-content`
//! reads back exactly what was written.
//!
//! Every value is a YAML double-quoted string with JSON's escapes, which
//! YAML reads the same way, so no title, however odd, can change the shape
//! of the file. Only `true` is written bare.

use std::fmt::Write as _;

/// Front matter in the order its fields are added.
#[derive(Debug, Default)]
pub(crate) struct FrontMatter {
    yaml: String,
}

impl FrontMatter {
    /// Adds `key` with `value` as quoted text.
    pub(crate) fn text(&mut self, key: &str, value: &str) -> &mut Self {
        self.yaml.push_str(key);
        self.yaml.push_str(": ");
        self.yaml.push_str(&quoted(value));
        self.yaml.push('\n');
        self
    }

    /// Adds `key` with the text of `value`, when there is one.
    pub(crate) fn maybe(&mut self, key: &str, value: Option<impl ToString>) -> &mut Self {
        if let Some(value) = value {
            self.text(key, &value.to_string());
        }
        self
    }

    /// Adds `key` as a list, when `values` holds any.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "aliases use it from task 6.1 part 5")
    )]
    pub(crate) fn list(&mut self, key: &str, values: &[String]) -> &mut Self {
        if !values.is_empty() {
            self.yaml.push_str(key);
            self.yaml.push_str(":\n");
            for value in values {
                self.yaml.push_str("  - ");
                self.yaml.push_str(&quoted(value));
                self.yaml.push('\n');
            }
        }
        self
    }

    /// Adds `key: true`.
    pub(crate) fn yes(&mut self, key: &str) -> &mut Self {
        self.yaml.push_str(key);
        self.yaml.push_str(": true\n");
        self
    }

    /// The whole file: the front matter between its fences, a blank line,
    /// and `body`.
    pub(crate) fn file(&self, body: &str) -> String {
        format!("---\n{}---\n\n{body}", self.yaml)
    }
}

/// `value` in double quotes, with every character YAML could misread
/// escaped the way JSON escapes it.
pub(crate) fn quoted(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len().saturating_add(2));
    quoted.push('"');
    for c in value.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            '\r' => quoted.push_str("\\r"),
            c if c.is_control() || c == '\u{feff}' => {
                // Writing to a String cannot fail.
                let _ = write!(quoted, "\\u{:04x}", u32::from(c));
            }
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use khalid_content::parse_post;
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn a_post_reads_back_field_by_field() {
        let mut front = FrontMatter::default();
        front
            .text("title", "Rye: \"slow\" bread")
            .text("slug", "rye-slow-bread")
            .text("date", "2024-03-12")
            .maybe("author", Some("mara-lind"))
            .maybe("category", None::<&str>)
            .list("aliases", &["/2024/03/rye/".to_owned()])
            .yes("featured")
            .text("robots", "noindex, nofollow");
        let text = front.file("Rye needs time.\n");
        let post = parse_post(Path::new("post.md"), &text).unwrap();
        assert_eq!(post.front.title.as_deref(), Some("Rye: \"slow\" bread"));
        assert_eq!(
            post.front.slug.map(|slug| slug.to_string()).as_deref(),
            Some("rye-slow-bread")
        );
        assert_eq!(
            post.front.date.map(|date| date.to_string()).as_deref(),
            Some("2024-03-12")
        );
        assert_eq!(post.front.category, None);
        assert_eq!(post.front.aliases, ["/2024/03/rye/"]);
        assert!(post.front.featured);
        assert_eq!(
            post.front
                .robots
                .map(|robots| robots.to_string())
                .as_deref(),
            Some("noindex, nofollow")
        );
        assert_eq!(post.body, "\nRye needs time.\n");
    }

    #[test]
    fn an_empty_list_is_left_out() {
        let text = FrontMatter::default().list("aliases", &[]).file("");
        assert_eq!(text, "---\n---\n\n");
    }

    proptest! {
        #[test]
        fn any_title_reads_back_as_written(title in any::<String>()) {
            let text = FrontMatter::default().text("title", &title).file("");
            let post = parse_post(Path::new("post.md"), &text);
            prop_assert_eq!(post.ok().and_then(|post| post.front.title), Some(title));
        }
    }
}
