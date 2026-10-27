//! The Ghost cards that become one of Khalid's own blocks, such as a
//! bookmark or a toggle (ADR 0012, ADR 0018). A block whose line does not
//! read fails the whole post, so every address is checked first, and a
//! card that cannot become a block is left out and named.

use khalid_core::AbsoluteUrl;
use serde_json::Value;

use crate::markdown::is_safe;
use crate::pieces::Html;

/// Cards with no place in Khalid: members, newsletters, products and
/// podcasts' own players.
const LEFT: [&str; 7] = [
    "product",
    "signup",
    "paywall",
    "email",
    "email-cta",
    "transistor",
    "call-to-action",
];

/// Writes the card `kind`, whose fields are in `card`, as a Khalid block,
/// and says whether `kind` is one of these cards.
pub(crate) fn block_card(kind: &str, card: &Value, html: &mut Html) -> bool {
    let field = |name: &str| text(card, name);
    let written = match kind {
        "embed" => web(field("url")).is_some_and(|src| {
            let title = text(&card["metadata"], "title").unwrap_or(src.host());
            html.block("embed", &[("src", src.as_str()), ("title", title)], None);
            true
        }),
        "bookmark" => web(field("url")).is_some_and(|href| {
            let meta = &card["metadata"];
            let image = web(text(meta, "thumbnail"));
            let mut attributes = vec![("href", href.as_str())];
            attributes.extend(text(meta, "title").map(|title| ("title", title)));
            attributes.extend(text(meta, "description").map(|about| ("description", about)));
            attributes.extend(image.as_ref().map(|image| ("image", image.as_str())));
            html.block("bookmark", &attributes, None);
            true
        }),
        "callout" => {
            let emoji = field("calloutEmoji").unwrap_or_default();
            let words = field("calloutText").unwrap_or_default();
            html.block("callout", &[], Some(format!("{emoji} {words}").trim()));
            true
        }
        "button" => match (
            field("buttonUrl").filter(|href| is_safe(href)),
            field("buttonText"),
        ) {
            (Some(href), Some(label)) => {
                html.block("button", &[("href", href), ("label", label)], None);
                true
            }
            _ => false,
        },
        "toggle" => {
            let summary = plain(field("heading").unwrap_or_default());
            let content = field("content").unwrap_or_default();
            if summary.is_empty() {
                html.tag(content);
            } else {
                html.block("toggle", &[("summary", &summary)], Some(content));
            }
            true
        }
        "gallery" => gallery(card, html),
        "header" => {
            let heading = field("header").unwrap_or_default();
            let body = format!(
                "<h2>{heading}</h2>{}",
                field("subheader").unwrap_or_default()
            );
            let image = field("backgroundImageSrc").filter(|src| media(src));
            let alt = plain(heading);
            let attributes: Vec<(&str, &str)> =
                image.map_or_else(Vec::new, |src| vec![("image", src), ("alt", &alt)]);
            html.block("header", &attributes, Some(&body));
            true
        }
        "audio" | "file" | "video" => file(kind, card, html),
        _ if LEFT.contains(&kind) => false,
        _ => return false,
    };
    if !written {
        html.leave(kind);
    }
    true
}

/// A gallery's pictures, one `![alt](src)` per line, or nothing when none
/// has a safe address.
fn gallery(card: &Value, html: &mut Html) -> bool {
    let images = card.get("images").and_then(Value::as_array);
    let lines: Vec<String> = images
        .into_iter()
        .flatten()
        .filter_map(|image| {
            let src =
                text(image, "src").filter(|src| media(src) && !src.contains([' ', '(', ')']))?;
            // A gallery's line needs alt text: the caption, else the file's
            // name, stands in when Ghost has none.
            let caption = text(image, "caption").map(plain);
            let named = text(image, "alt").map(str::to_owned).or(caption);
            let alt: String = named
                .filter(|alt| !alt.trim().is_empty())
                .unwrap_or_else(|| src.rsplit('/').next().unwrap_or(src).to_owned())
                .chars()
                .filter(|c| !matches!(c, '[' | ']') && !c.is_control())
                .collect();
            Some(format!("![{}]({src})", alt.trim()))
        })
        .collect();
    if lines.is_empty() {
        return false;
    }
    html.lines(format!(":::gallery\n{}\n:::", lines.join("\n")));
    true
}

/// A sound, a download or a video. A video needs the still picture shown
/// before it plays, which Ghost calls its thumbnail.
fn file(kind: &str, card: &Value, html: &mut Html) -> bool {
    let Some(src) = text(card, "src").filter(|src| media(src)) else {
        return false;
    };
    let named = ["title", "fileTitle", "fileName", "caption"]
        .iter()
        .find_map(|name| text(card, name).map(plain))
        .filter(|title| !title.is_empty());
    let last = src.rsplit('/').next().unwrap_or(src);
    let title = named.unwrap_or_else(|| last.to_owned());
    if kind != "video" {
        html.block(kind, &[("src", src), ("title", &title)], None);
        return true;
    }
    match text(card, "thumbnailSrc").filter(|poster| media(poster)) {
        Some(poster) => {
            let attributes = [("src", src), ("title", title.as_str()), ("poster", poster)];
            html.block("video", &attributes, None);
            true
        }
        None => false,
    }
}

/// The text of the field `name` of `value`, unless it is blank.
fn text<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
}

/// A full web address, the only kind a bookmark or an embed takes.
fn web(text: Option<&str>) -> Option<AbsoluteUrl> {
    AbsoluteUrl::parse(text?).ok()
}

/// A path on the site or a full web address, as media blocks take.
fn media(src: &str) -> bool {
    is_safe(src) && (!src.contains(':') || AbsoluteUrl::parse(src).is_ok())
}

/// The words of a piece of HTML, such as a toggle's heading, without its
/// tags, with the common entities read and its spaces collapsed.
fn plain(html: &str) -> String {
    let mut words = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => words.push(c),
            _ => {}
        }
    }
    let words = words
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use khalid_content::{BlockKind, parse_blocks};
    use khalid_core::{Locale, SlugMode};

    use crate::ghost::body::converted;
    use crate::ghost::lexical::lexical_html;

    /// The markdown of a Lexical document holding `nodes`, which must read
    /// back as blocks, with what it left out.
    fn markdown(nodes: &str) -> (String, Vec<String>) {
        let json = format!(r#"{{"root":{{"type":"root","children":[{nodes}]}}}}"#);
        let locale = Locale::try_new("en").unwrap();
        let body = converted(lexical_html(&json).unwrap(), &locale, SlugMode::Ascii).unwrap();
        parse_blocks(&body.markdown).unwrap();
        (body.markdown, body.left)
    }

    #[test]
    fn embeds_and_bookmarks_keep_their_addresses_and_titles() {
        let (text, _) = markdown(concat!(
            r#"{"type":"embed","url":"https://www.youtube.com/watch?v=abc","html":"<iframe></iframe>","metadata":{"title":"Shaping \"rye\""}},"#,
            r#"{"type":"bookmark","url":"https://flour.example.net/guide/","metadata":{"title":"Flour guide","description":"Which flour {when}","thumbnail":"https://flour.example.net/t.jpg","icon":"x"}}"#,
        ));
        assert_eq!(
            text,
            "::embed{src=\"https://www.youtube.com/watch?v=abc\" title=\"Shaping rye\"}\n\n::bookmark{href=\"https://flour.example.net/guide/\" title=\"Flour guide\" description=\"Which flour when\" image=\"https://flour.example.net/t.jpg\"}\n"
        );
    }

    #[test]
    fn callouts_buttons_and_toggles_become_their_blocks() {
        let (text, _) = markdown(concat!(
            r#"{"type":"callout","calloutEmoji":"💡","calloutText":"<b>Feed</b> it daily."},"#,
            r#"{"type":"button","buttonText":"Order bread","buttonUrl":"https://northfield.example.com/order/"},"#,
            r#"{"type":"toggle","heading":"<span>Why rye?</span>","content":"<p>It keeps.</p>"}"#,
        ));
        let blocks = parse_blocks(&text).unwrap();
        let kinds: Vec<&BlockKind> = blocks.iter().map(|block| &block.kind).collect();
        assert!(matches!(kinds[0], BlockKind::Callout { .. }), "{text}");
        assert!(
            text.contains(":::callout\n💡 **Feed** it daily.\n:::"),
            "{text}"
        );
        assert!(matches!(kinds[1], BlockKind::Button { label, .. } if label == "Order bread"));
        assert!(matches!(kinds[2], BlockKind::Toggle { summary } if summary == "Why rye?"));
    }

    #[test]
    fn galleries_headers_and_media_become_their_blocks() {
        let (text, _) = markdown(concat!(
            r#"{"type":"gallery","images":[{"src":"https://northfield.example.com/a.jpg","alt":"A [loaf]"},{"src":"javascript:x"},{"src":"/content/images/b.jpg","alt":""}]},"#,
            r#"{"type":"header","header":"Rye week","subheader":"<p>Seven loaves</p>","backgroundImageSrc":"/content/images/week.jpg"},"#,
            r#"{"type":"audio","src":"/content/media/talk.mp3","title":"A talk"},"#,
            r#"{"type":"file","src":"/content/files/recipe.pdf","fileTitle":"","fileName":"recipe.pdf"},"#,
            r#"{"type":"video","src":"/content/media/shape.mp4","thumbnailSrc":"/content/images/shape.jpg","caption":"<i>Shaping</i>"}"#,
        ));
        assert!(
            text.starts_with(":::gallery\n![A loaf](https://northfield.example.com/a.jpg)\n![b.jpg](/content/images/b.jpg)\n:::\n\n:::header{image=\"/content/images/week.jpg\" alt=\"Rye week\"}\n## Rye week\n\nSeven loaves\n:::\n\n"),
            "{text}"
        );
        assert!(
            text.ends_with("::audio{src=\"/content/media/talk.mp3\" title=\"A talk\"}\n\n::file{src=\"/content/files/recipe.pdf\" title=\"recipe.pdf\"}\n\n::video{src=\"/content/media/shape.mp4\" title=\"Shaping\" poster=\"/content/images/shape.jpg\"}\n"),
            "{text}"
        );
    }

    #[test]
    fn cards_with_no_place_or_no_safe_address_are_left_out_and_named() {
        let (text, left) = markdown(concat!(
            r#"{"type":"signup","header":"Join"},{"type":"paywall"},"#,
            r#"{"type":"button","buttonText":"Run","buttonUrl":"javascript:steal()"},"#,
            r#"{"type":"bookmark","url":"not an address"},"#,
            r#"{"type":"video","src":"/content/media/a.mp4"}"#,
        ));
        assert_eq!(text, "\n");
        assert_eq!(left, ["signup", "paywall", "button", "bookmark", "video"]);
    }
}
