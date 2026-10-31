//! A post file's text, edited field by field, so every front matter line
//! Khalid does not change stays exactly as the writer left it.

use khalid_core::Slug;

/// A post file: its front matter lines and the body after the fence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostText {
    front: Vec<String>,
    body: String,
}

impl PostText {
    /// A new draft with a title, slug and author, and `markdown` as its body.
    pub(crate) fn draft(title: &str, slug: &Slug, author: Option<&Slug>, markdown: &str) -> Self {
        let mut text = Self {
            front: Vec::new(),
            body: String::new(),
        };
        text.set("title", &quoted(title));
        text.set("slug", slug.as_str());
        if let Some(author) = author {
            text.set("author", author.as_str());
        }
        text.set_body(markdown);
        text
    }

    /// Splits `text` at its front matter fences, or `None` when it has none.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let rest = text.strip_prefix("---\n")?;
        let (front, body) = match rest.strip_prefix("---\n") {
            Some(body) => ("", body),
            None => rest.split_once("\n---\n")?,
        };
        Some(Self {
            front: front.lines().map(str::to_owned).collect(),
            body: body.to_owned(),
        })
    }

    /// Sets the top-level field `key` to `value`, written as YAML on one
    /// line, in place when the field is there and last when it is not. A
    /// value the field had on the lines below it, such as a list, goes.
    pub(crate) fn set(&mut self, key: &str, value: &str) {
        let line = format!("{key}: {value}");
        match self.remove(key) {
            Some(at) => self.front.insert(at, line),
            None => self.front.push(line),
        }
    }

    /// Removes the top-level field `key` and the lines of its value, and
    /// returns where it was, or `None` when the post has no such field.
    pub(crate) fn remove(&mut self, key: &str) -> Option<usize> {
        let prefix = format!("{key}:");
        let at = self.front.iter().position(|l| l.starts_with(&prefix))?;
        let below = self.front.iter().skip(at + 1);
        let value_lines = below.take_while(|l| l.starts_with([' ', '-'])).count();
        self.front.drain(at..=at + value_lines);
        Some(at)
    }

    /// Replaces the body with `markdown`, after one blank line.
    pub(crate) fn set_body(&mut self, markdown: &str) {
        self.body = format!("\n{}\n", markdown.trim_end_matches('\n'));
    }

    /// The body as the writer sees it, without the blank line after the
    /// fence or the last line break.
    pub(crate) fn markdown(&self) -> &str {
        let body = self.body.strip_prefix('\n').unwrap_or(&self.body);
        body.trim_end_matches('\n')
    }

    /// The front matter between the fences, as YAML.
    pub(crate) fn front_matter(&self) -> String {
        self.front.join("\n")
    }

    /// The whole file.
    pub(crate) fn to_text(&self) -> String {
        format!("---\n{}\n---\n{}", self.front.join("\n"), self.body)
    }
}

/// `text` as a double-quoted YAML string on one line.
pub(crate) fn quoted(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edit_keeps_every_other_line_as_written() {
        let file = "---\ntitle: Old\ndescription: Kept: as is\n---\n\nBody.\n";
        let mut text = PostText::parse(file).unwrap();
        text.set("title", &quoted("New \"one\""));
        text.set("date", "2026-10-26");
        assert_eq!(
            text.to_text(),
            "---\ntitle: \"New \\\"one\\\"\"\ndescription: Kept: as is\ndate: 2026-10-26\n---\n\nBody.\n"
        );
        assert_eq!(text.markdown(), "Body.");
    }

    #[test]
    fn a_list_is_replaced_whole_and_a_field_can_go() {
        let file = "---\naliases:\n  - /old/\n  - /older/\ntitle: T\n---\nBody.\n";
        let mut text = PostText::parse(file).unwrap();
        text.set("aliases", "[\"/new/\"]");
        assert_eq!(
            text.to_text(),
            "---\naliases: [\"/new/\"]\ntitle: T\n---\nBody.\n"
        );
        assert_eq!(text.remove("title"), Some(1));
        assert_eq!(text.remove("featured"), None);
        assert_eq!(text.to_text(), "---\naliases: [\"/new/\"]\n---\nBody.\n");
    }

    #[test]
    fn a_draft_reads_back_as_written() {
        let slug = Slug::try_new("anchor-text").unwrap();
        let author = Slug::try_new("asif").unwrap();
        let draft = PostText::draft("Anchor text", &slug, Some(&author), "Words.\n");
        let again = PostText::parse(&draft.to_text()).unwrap();
        assert_eq!(again, draft);
        assert_eq!(again.markdown(), "Words.");
        assert!(PostText::parse("no fence").is_none());
    }
}
