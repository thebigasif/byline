//! A Lexical document, the editor format of Ghost 5 and later, written as
//! HTML for the markdown converter.

use serde_json::Value;

use crate::ghost::cards::card;
use crate::ghost::html::Html;

/// The bits of a text node's `format` that the markdown converter carries,
/// with the element each one opens, outermost first. Strikethrough (4),
/// underline (8), subscript (32) and superscript (64) keep their words and
/// lose the format.
const FORMATS: [(u64, &str); 3] = [(1, "strong"), (2, "em"), (16, "code")];

/// The heading elements a heading node may name; any other becomes `h2`.
const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];

/// The HTML of the Lexical document `json`.
///
/// # Errors
///
/// Returns the JSON reader's error when `json` is not JSON.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the Ghost plan, a later part, converts bodies")
)]
pub(crate) fn lexical_html(json: &str) -> Result<Html, serde_json::Error> {
    let document: Value = serde_json::from_str(json)?;
    let mut html = Html::default();
    if let Some(root) = document.get("root") {
        children(root, &mut html);
    }
    Ok(html)
}

/// Writes each child of `node` in order.
fn children(node: &Value, html: &mut Html) {
    let children = node.get("children").and_then(Value::as_array);
    for child in children.into_iter().flatten() {
        write_node(child, html);
    }
}

/// Writes one node: an element around its children, a piece of text, or,
/// for a kind with no place in Khalid, its text alone when it has some.
fn write_node(node: &Value, html: &mut Html) {
    let field = |name: &str| node.get(name).and_then(Value::as_str);
    let kind = field("type").unwrap_or_default();
    let element = match kind {
        "paragraph" => "p",
        "heading" | "extended-heading" => field("tag")
            .filter(|tag| HEADINGS.contains(tag))
            .unwrap_or("h2"),
        "quote" | "extended-quote" | "aside" => "blockquote",
        "list" if field("listType") == Some("number") => "ol",
        "list" => "ul",
        "listitem" => "li",
        "link" | "autolink" => {
            html.tag("<a href=\"");
            html.escaped(field("url").unwrap_or_default());
            html.tag("\">");
            children(node, html);
            html.tag("</a>");
            return;
        }
        "text" | "extended-text" => {
            let format = node.get("format").and_then(Value::as_u64).unwrap_or(0);
            text(field("text").unwrap_or_default(), format, html);
            return;
        }
        "linebreak" => {
            html.tag("<br>");
            return;
        }
        "tab" => {
            html.tag(" ");
            return;
        }
        _ => {
            if !card(kind, node, html) {
                if node.get("children").is_none() {
                    html.leave(kind);
                }
                children(node, html);
            }
            return;
        }
    };
    html.tag(&format!("<{element}>"));
    children(node, html);
    html.tag(&format!("</{element}>"));
}

/// Writes `value` inside the elements its `format` bits ask for.
fn text(value: &str, format: u64, html: &mut Html) {
    let wanted: Vec<&str> = FORMATS
        .iter()
        .filter(|(bit, _)| format & bit != 0)
        .map(|(_, element)| *element)
        .collect();
    for element in &wanted {
        html.tag(&format!("<{element}>"));
    }
    html.escaped(value);
    for element in wanted.iter().rev() {
        html.tag(&format!("</{element}>"));
    }
}

#[cfg(test)]
mod tests {
    use khalid_core::{Locale, SlugMode};

    use super::*;
    use crate::ghost::body::converted;

    fn markdown(nodes: &str) -> String {
        let json = format!(r#"{{"root":{{"type":"root","children":[{nodes}]}}}}"#);
        let html = lexical_html(&json).unwrap();
        let locale = Locale::try_new("en").unwrap();
        converted(html, &locale, SlugMode::Ascii).unwrap().markdown
    }

    fn text(value: &str, format: u8) -> String {
        format!(r#"{{"type":"extended-text","text":"{value}","format":{format}}}"#)
    }

    fn block(kind: &str, extra: &str, children: &[String]) -> String {
        let children = children.join(",");
        format!(r#"{{"type":"{kind}"{extra},"children":[{children}]}}"#)
    }

    #[test]
    fn paragraphs_headings_and_inline_formats_become_markdown() {
        let link = block(
            "link",
            r#","url":"https://northfield.example.com/rye/""#,
            &[text("rye", 0)],
        );
        let nodes = [
            block("extended-heading", r#","tag":"h3""#, &[text("Feeding", 0)]),
            block(
                "paragraph",
                "",
                &[
                    text("Feed it ", 0),
                    text("every day", 3),
                    text(", see ", 0),
                    link,
                ],
            ),
            block(
                "paragraph",
                "",
                &[
                    text("flour", 16),
                    r#"{"type":"linebreak"}"#.to_owned(),
                    text("water", 4),
                ],
            ),
        ];
        assert_eq!(
            markdown(&nodes.join(",")),
            "### Feeding\n\nFeed it ***every day***, see [rye](https://northfield.example.com/rye/)\n\n`flour`  \nwater\n"
        );
    }

    #[test]
    fn quotes_and_nested_lists_keep_their_shape() {
        let item = |words: &str| block("listitem", "", &[text(words, 0)]);
        let steps = block(
            "list",
            r#","listType":"number""#,
            &[item("Mix"), item("Wait")],
        );
        let nested = block("listitem", "", &[text("Rye", 0), steps]);
        let nodes = [
            block("extended-quote", "", &[text("Time is an ingredient.", 0)]),
            block("list", r#","listType":"bullet""#, &[nested, item("Spelt")]),
        ];
        let markdown = markdown(&nodes.join(","));
        assert!(
            markdown.starts_with("> Time is an ingredient.\n\n"),
            "{markdown:?}"
        );
        assert!(
            markdown.contains("Rye\n    1.  Mix\n    2.  Wait\n"),
            "{markdown:?}"
        );
        assert!(markdown.ends_with("Spelt\n"), "{markdown:?}");
    }

    #[test]
    fn text_is_never_markup_and_unsafe_links_keep_only_their_words() {
        let link = block("link", r#","url":"javascript:steal()""#, &[text("tips", 0)]);
        let nodes = [block(
            "paragraph",
            "",
            &[text("Use <script> tags ", 0), link],
        )];
        assert_eq!(markdown(&nodes.join(",")), "Use \\<script> tags tips\n");
        let odd = block("heading", r#","tag":"script""#, &[text("Proof", 0)]);
        assert_eq!(
            markdown(&odd),
            "## Proof\n",
            "an unknown tag is a level 2 heading"
        );
    }

    #[test]
    fn a_kind_khalid_has_no_place_for_is_named_and_its_text_kept() {
        let json = format!(
            r#"{{"root":{{"children":[{},{{"type":"signup","header":"Join"}}]}}}}"#,
            block("mystery", "", &[text("kept words", 0)])
        );
        let html = lexical_html(&json).unwrap();
        let locale = Locale::try_new("en").unwrap();
        let body = converted(html, &locale, SlugMode::Ascii).unwrap();
        assert_eq!(
            (body.markdown.as_str(), body.left),
            ("kept words\n", vec!["signup".to_owned()])
        );
        assert!(lexical_html("{\"root\":").is_err());
    }
}
