//! Escaping for text placed in HTML and XML.

/// Escapes `&`, `<`, `>`, `"` and `'`, so the text is safe in element
/// content and in a quoted attribute, in both HTML and XML.
pub(crate) fn escape_markup(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::escape_markup;

    #[test]
    fn escapes_every_character_that_can_end_an_attribute_or_open_a_tag() {
        let mut out = String::new();
        escape_markup(r#"Tom & "Jerry's" <b>"#, &mut out);
        assert_eq!(out, "Tom &amp; &quot;Jerry&#39;s&quot; &lt;b&gt;");
    }
}
