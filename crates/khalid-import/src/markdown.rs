//! A post body from another system's HTML, made safe and written as
//! markdown (RULES.md BL-SEC-1).

use htmd::element_handler::{HandlerResult, Handlers};
use htmd::{Element, HtmlToMarkdown};
use khalid_core::AbsoluteUrl;

/// A body as markdown, with what the conversion could not turn into
/// Khalid's own markup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Body {
    /// The markdown.
    pub(crate) markdown: String,
    /// The names of the shortcodes left as text, such as `gallery`, in the
    /// order they first appear.
    pub(crate) shortcodes: Vec<String>,
}

/// Elements dropped with everything inside them: they run code or collect
/// input, and a post body needs none of that.
const DROPPED: [&str; 8] = [
    "script", "style", "object", "embed", "form", "noscript", "template", "svg",
];

/// Starts a line the converter wrote as a block itself, such as an
/// `::embed`, so the escaping of text that looks like a block leaves it
/// alone. It is a private-use character, removed from the HTML first, so
/// no post can forge one.
const OWN_BLOCK: char = '\u{e000}';

/// The schemes a link or image may keep.
const SCHEMES: [&str; 3] = ["http", "https", "mailto"];

/// Tags that start a block, so the text around them is not a paragraph.
const BLOCKS: [&str; 16] = [
    "<p",
    "<div",
    "<h1",
    "<h2",
    "<h3",
    "<h4",
    "<h5",
    "<h6",
    "<ul",
    "<ol",
    "<blockquote",
    "<figure",
    "<table",
    "<pre",
    "<iframe",
    "<!--",
];

/// The markdown for `html`. `ammonia` cleans the HTML against its allowlist
/// first, keeping frames only for their address and title. A frame becomes
/// an `::embed` block; a link or image keeps only a safe address. A line
/// that would read as a block directive is escaped.
///
/// # Errors
///
/// Returns the converter's error when the HTML cannot be read.
pub(crate) fn body_markdown(html: &str) -> std::io::Result<Body> {
    let html = paragraphs(&html.replace("\r\n", "\n").replace(OWN_BLOCK, ""));
    let clean = ammonia::Builder::default()
        .add_tags(["iframe"])
        .add_tag_attributes("iframe", ["src", "title"])
        .url_schemes(SCHEMES.into_iter().collect())
        .clean(&html)
        .to_string();
    let converter = HtmlToMarkdown::builder()
        .skip_tags(DROPPED.to_vec())
        .add_handler(vec!["a", "img", "iframe"], safe_address)
        .build();
    let markdown = converter.convert(&clean)?;
    let lines: Vec<String> = markdown
        .trim()
        .lines()
        .map(|line| {
            // `::` starts a block in Khalid; WordPress text never meant one.
            if let Some(own) = line.trim_start().strip_prefix(OWN_BLOCK) {
                format!("::{own}")
            } else if line.trim_start().starts_with("::") {
                format!("\\{}", line.trim_start())
            } else {
                line.to_owned()
            }
        })
        .collect();
    Ok(Body {
        markdown: format!("{}\n", lines.join("\n")),
        shortcodes: shortcodes(&html),
    })
}

/// Wraps text the classic editor separated with blank lines in paragraphs,
/// as `WordPress` does when it shows a post, and turns its single line
/// breaks into `<br>`. Blocks, and anything inside a `<pre>`, stay as they
/// are.
fn paragraphs(html: &str) -> String {
    let mut out = Vec::new();
    let mut open_pre = 0_usize;
    for chunk in html.split("\n\n") {
        let lower = chunk.to_ascii_lowercase();
        let trimmed = chunk.trim();
        let is_block = BLOCKS.iter().any(|tag| lower.trim_start().starts_with(tag));
        if open_pre > 0 {
            // Inside code every space and every blank line counts.
            out.push(chunk.to_owned());
        } else if trimmed.is_empty() {
            continue;
        } else if is_block {
            out.push(trimmed.to_owned());
        } else {
            out.push(format!("<p>{}</p>", trimmed.replace('\n', "<br>\n")));
        }
        open_pre = (open_pre + lower.matches("<pre").count())
            .saturating_sub(lower.matches("</pre>").count());
    }
    out.join("\n\n")
}

/// A frame as an `::embed` block, or nothing when its address is not one.
fn embed(element: &Element<'_>) -> Option<HandlerResult> {
    let attrs = &element.attrs;
    let attribute = |name: &str| {
        let found = attrs.iter().find(|a| &*a.name.local == name);
        found.map(|a| a.value.to_string())
    };
    let src = AbsoluteUrl::parse(&attribute("src")?).ok()?;
    // Quotes and braces would end the attribute; the frame's title is
    // plain words.
    let title: String = attribute("title")
        .unwrap_or_else(|| src.host().to_owned())
        .chars()
        .filter(|c| !matches!(c, '"' | '{' | '}' | '\\'))
        .collect();
    Some(
        format!(
            "\n\n{OWN_BLOCK}embed{{src=\"{src}\" title=\"{}\"}}\n\n",
            title.trim()
        )
        .into(),
    )
}

/// Turns a frame into an embed. Keeps a link or image whose address is
/// safe; otherwise keeps only a link's text and drops the image.
fn safe_address(handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
    if element.tag == "iframe" {
        return embed(&element);
    }
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

/// `http`, `https`, `mailto`, a relative address or a `#` anchor; never an
/// empty address, which leads nowhere.
pub(crate) fn is_safe(address: &str) -> bool {
    let address = address.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| address.starts_with(scheme))
        || (!address.is_empty() && !address.contains(':'))
}

/// The names of the shortcodes in `html`, such as `caption` in
/// `[caption id="1"]`, each once.
fn shortcodes(html: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for (_, rest) in html
        .match_indices('[')
        .filter_map(|(i, _)| html.split_at_checked(i))
    {
        let name: String = rest
            .chars()
            .skip(1)
            .take_while(|c| c.is_ascii_lowercase() || *c == '_' || *c == '-')
            .collect();
        let after = rest.chars().nth(name.chars().count().saturating_add(1));
        if !name.is_empty() && matches!(after, Some(' ' | ']')) && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use khalid_content::parse_blocks;

    use super::*;

    fn markdown(html: &str) -> String {
        body_markdown(html).unwrap().markdown
    }

    #[test]
    fn block_editor_markup_becomes_plain_markdown() {
        let html = "<!-- wp:paragraph -->\n<p>Feed it <strong>every day</strong>.</p>\n<!-- /wp:paragraph -->\n\n<!-- wp:heading -->\n<h2>Warmth</h2>\n<!-- /wp:heading -->";
        assert_eq!(markdown(html), "Feed it **every day**.\n\n## Warmth\n");
    }

    #[test]
    fn classic_editor_paragraphs_and_line_breaks_are_kept() {
        let html = "Rye needs a wetter dough.\n\nMix it,\nthen wait.\r\n\r\n<h2>Baking</h2>";
        let markdown = markdown(html);
        assert!(
            markdown.starts_with("Rye needs a wetter dough.\n\nMix it,"),
            "{markdown:?}"
        );
        assert!(markdown.contains("then wait.\n\n## Baking"), "{markdown:?}");
    }

    #[test]
    fn code_keeps_its_blank_lines() {
        let markdown = markdown("<pre><code>mix\n\nrest</code></pre>");
        assert!(markdown.contains("mix\n\nrest"), "{markdown:?}");
        assert!(!markdown.contains("<p>"), "{markdown:?}");
        let indented = self::markdown("<pre><code>if a {\n\n    feed();\n\n\n\n}</code></pre>");
        assert!(
            indented.contains("if a {\n\n    feed();\n\n\n\n}"),
            "indentation and every blank line are kept: {indented:?}"
        );
    }

    #[test]
    fn a_frame_becomes_an_embed_block() {
        let html = r#"<p>Watch first.</p><iframe src="https://www.youtube.com/embed/dQw4w9WgXcQ" title="Shaping a &quot;boule&quot;" width="560"></iframe>"#;
        let markdown = markdown(html);
        assert!(
            markdown.lines().any(|line| line == "::embed{src=\"https://www.youtube.com/embed/dQw4w9WgXcQ\" title=\"Shaping a boule\"}"),
            "{markdown:?}"
        );
        let blocks = parse_blocks(&markdown).unwrap();
        assert!(
            blocks
                .iter()
                .any(|block| matches!(block.kind, khalid_content::BlockKind::Embed { .. })),
            "{blocks:?}"
        );
    }

    #[test]
    fn scripts_and_unsafe_addresses_leave_only_text() {
        let html =
            r#"<p><script>steal()</script><a href="javascript:steal()">Starter tips</a></p>"#;
        assert_eq!(markdown(html), "Starter tips\n");
        assert_eq!(markdown(r#"<p><a href="">Nowhere</a></p>"#), "Nowhere\n");
    }

    #[test]
    fn shortcodes_are_left_as_text_and_named() {
        let body = body_markdown(
            r#"[caption id="a1"]A jar[/caption] [gallery ids="1,2"] [gallery] costs [5]"#,
        )
        .unwrap();
        assert_eq!(body.shortcodes, ["caption", "gallery"]);
        assert!(body.markdown.contains("A jar"), "{:?}", body.markdown);
    }

    #[test]
    fn text_that_looks_like_a_block_is_escaped() {
        let markdown = markdown("<p>::toc</p><p>:::note</p>");
        assert_eq!(markdown, "\\::toc\n\n\\:::note\n");
        assert!(parse_blocks(&markdown).is_ok());
        let forged = self::markdown("<p>\u{e000}toc</p>");
        assert_eq!(
            forged, "toc\n",
            "a post cannot mark its own text as a block"
        );
    }
}
