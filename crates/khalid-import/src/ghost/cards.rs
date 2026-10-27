//! The cards Ghost's two editors share, such as images and code, written as
//! HTML or as Khalid's own blocks. Lexical keeps a card's fields on its
//! node; Mobiledoc keeps them in a payload. Both look the same here.

use serde_json::Value;

use crate::ghost::html::Html;
use crate::markdown::is_safe;

/// Writes the card `kind`, whose fields are in `card`, and says whether
/// `kind` names a card at all.
pub(crate) fn card(kind: &str, card: &Value, html: &mut Html) -> bool {
    let field = |name: &str| {
        card.get(name)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
    };
    match kind {
        "image" => match field("src").filter(|src| is_safe(src)) {
            // The caption is HTML, which the converter cleans.
            Some(src) => {
                let alt = field("alt").unwrap_or_default();
                let caption = field("caption").unwrap_or_default();
                html.block("figure", &[("src", src), ("alt", alt)], Some(caption));
            }
            None => html.leave(kind),
        },
        "codeblock" | "code" => {
            let code = card.get("code").and_then(Value::as_str).unwrap_or_default();
            html.code(field("language").unwrap_or_default(), code);
        }
        "markdown" | "card-markdown" => html.markdown(field("markdown").unwrap_or_default()),
        // The converter cleans it with `ammonia` like any other HTML.
        "html" => html.tag(field("html").unwrap_or_default()),
        "horizontalrule" | "hr" => html.tag("<hr>"),
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use khalid_content::{BlockKind, parse_blocks};
    use khalid_core::{Locale, SlugMode};

    use crate::ghost::body::converted;
    use crate::ghost::lexical::lexical_html;

    /// The markdown of a Lexical document holding `nodes`, with what it
    /// left out.
    fn markdown(nodes: &str) -> (String, Vec<String>) {
        let json = format!(r#"{{"root":{{"type":"root","children":[{nodes}]}}}}"#);
        let locale = Locale::try_new("en").unwrap();
        let html = lexical_html(&json).unwrap();
        let body = converted(html, &locale, SlugMode::Ascii).unwrap();
        (body.markdown, body.left)
    }

    #[test]
    fn an_image_becomes_a_figure_with_its_caption() {
        let (markdown, _) = markdown(
            r#"{"type":"image","src":"https://northfield.example.com/content/images/rye.jpg","alt":"A \"dark\" loaf","caption":"<span>Baked at <b>dawn</b></span>"}"#,
        );
        assert_eq!(
            markdown,
            ":::figure{src=\"https://northfield.example.com/content/images/rye.jpg\" alt=\"A dark loaf\"}\nBaked at **dawn**\n:::\n"
        );
        let blocks = parse_blocks(&markdown).unwrap();
        assert!(
            matches!(blocks[0].kind, BlockKind::Figure { .. }),
            "{blocks:?}"
        );
    }

    #[test]
    fn an_image_with_an_unsafe_address_is_left_out_and_named() {
        let (markdown, left) = markdown(r#"{"type":"image","src":"javascript:steal()"}"#);
        assert_eq!((markdown.as_str(), left), ("\n", vec!["image".to_owned()]));
    }

    #[test]
    fn code_keeps_its_language_and_its_text_exactly() {
        let (rust, _) = markdown(
            r#"{"type":"codeblock","code":"if a < b {\n\n  feed();\n}","language":"rust\"><script>"}"#,
        );
        assert_eq!(rust, "```rustscript\nif a < b {\n\n  feed();\n}\n```\n");
        let (fenced, _) = markdown(r#"{"type":"code","code":"Use ``` to fence.\n::toc"}"#);
        assert_eq!(fenced, "````\nUse ``` to fence.\n::toc\n````\n");
        let blocks = parse_blocks(&fenced).unwrap();
        assert!(matches!(blocks[..], [ref code] if matches!(code.kind, BlockKind::Code { .. })));
    }

    #[test]
    fn a_markdown_card_is_rendered_and_cleaned_like_html() {
        let (markdown, _) = markdown(
            r###"{"type":"markdown","markdown":"## Steps\n\n1. Mix\n2. [Wait](javascript:steal())\n\n::toc\n\n<script>steal()</script>"}"###,
        );
        assert!(
            markdown.starts_with("## Steps\n\n1.  Mix\n2.  Wait\n"),
            "{markdown:?}"
        );
        assert!(!markdown.contains("javascript"), "{markdown:?}");
        assert!(markdown.contains("\\::toc"), "{markdown:?}");
        assert!(
            markdown.contains("\\<script>"),
            "raw HTML stays text: {markdown:?}"
        );
        assert!(parse_blocks(&markdown).is_ok());
    }

    #[test]
    fn an_html_card_is_cleaned_and_a_divider_kept() {
        let (markdown, _) = markdown(
            r#"{"type":"html","html":"<p onclick=\"steal()\">Hello <iframe src=\"https://www.youtube.com/embed/abc\" title=\"Rye\"></iframe></p>"},{"type":"horizontalrule"},{"type":"paragraph","children":[{"type":"text","text":"After"}]}"#,
        );
        assert!(!markdown.contains("onclick"), "{markdown:?}");
        assert!(
            markdown
                .contains("\n::embed{src=\"https://www.youtube.com/embed/abc\" title=\"Rye\"}\n"),
            "{markdown:?}"
        );
        assert!(markdown.ends_with("* * *\n\nAfter\n"), "{markdown:?}");
    }
}
