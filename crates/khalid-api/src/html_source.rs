//! HTML from another tool, made safe and turned into markdown.

use htmd::element_handler::{HandlerResult, Handlers};
use htmd::{Element, HtmlToMarkdown};

/// Elements dropped with everything inside them: they run code, embed
/// other pages, or collect input, and a post body needs none of that.
const DROPPED: [&str; 9] = [
    "script", "style", "iframe", "object", "embed", "form", "noscript", "template", "svg",
];

/// The markdown for `html` (RULES.md BL-SEC-1). Scripts, frames, forms and
/// their contents are dropped; a link or image keeps only an `http`,
/// `https`, `mailto`, relative or `#` address, and any other loses its
/// address but keeps its text. Markdown has no attributes, so event
/// handlers and styles cannot survive. Content files hold markdown, and
/// rendering escapes raw HTML in it again.
///
/// # Errors
///
/// Returns the converter's error when the HTML cannot be read.
pub(crate) fn markdown_from_html(html: &str) -> std::io::Result<String> {
    let converter = HtmlToMarkdown::builder()
        .skip_tags(DROPPED.to_vec())
        .add_handler(vec!["a", "img"], safe_address)
        .build();
    converter
        .convert(html)
        .map(|markdown| markdown.trim().to_owned())
}

/// Keeps a link or image whose address is safe; otherwise keeps only a
/// link's text and drops the image.
fn safe_address(handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
    let name = if element.tag == "img" { "src" } else { "href" };
    let address = element
        .attrs
        .iter()
        .find(|attribute| &*attribute.name.local == name)
        .map(|attribute| &*attribute.value);
    if address.is_none_or(is_safe) {
        handlers.fallback(element)
    } else if element.tag == "img" {
        None
    } else {
        Some(handlers.walk_children(element.node))
    }
}

/// `http`, `https`, `mailto`, a relative address or a `#` anchor. Any
/// other scheme, however spelled, is unsafe.
fn is_safe(address: &str) -> bool {
    let address = address.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| address.starts_with(scheme))
        || !address.contains(':')
}

#[cfg(test)]
mod tests {
    use super::{is_safe, markdown_from_html};

    #[test]
    fn lists_quotes_and_code_survive() {
        let html = "<ul><li>Links</li><li>Anchors</li></ul><blockquote><p>Link with care.</p></blockquote><pre><code>khalid lint</code></pre>";
        let markdown = markdown_from_html(html).unwrap();
        for part in ["Links", "Anchors", "> Link with care.", "khalid lint"] {
            assert!(markdown.contains(part), "{part} in {markdown:?}");
        }
    }

    #[test]
    fn dropped_elements_leave_nothing_behind() {
        let markdown = markdown_from_html(
            "<style>p { color: red }</style><svg><script>x()</script></svg><p>Kept.</p>",
        )
        .unwrap();
        assert_eq!(markdown, "Kept.");
    }

    #[test]
    fn only_safe_addresses_are_kept() {
        for safe in [
            "https://example.com/",
            "HTTP://example.com",
            "mailto:a@example.com",
            "/blog/",
            "#notes",
            "notes/",
        ] {
            assert!(is_safe(safe), "{safe}");
        }
        for unsafe_address in [
            "javascript:x()",
            " JavaScript:x()",
            "java\tscript:x()",
            "data:text/html,x",
            "vbscript:x",
        ] {
            assert!(!is_safe(unsafe_address), "{unsafe_address:?}");
        }
        let markdown =
            markdown_from_html("<p><img src=\"data:image/png;base64,x\" alt=\"pixel\">Text.</p>")
                .unwrap();
        assert_eq!(markdown, "Text.");
    }
}
