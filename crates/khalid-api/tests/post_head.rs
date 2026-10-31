//! A post's head in API shape (KH-INS-5): a front end that prints each tag
//! in order, then the JSON-LD in a script tag, writes the theme's head.

use khalid_api::resources::{HeadElement, PostHead};
use khalid_core::{AbsoluteUrl, Locale};
use khalid_seo::{PageKind, SeoPage, SeoSite, UrlPath, head_tags, page_graph};
use serde_json::{Value, json};

/// A post page whose title and summary need escaping, with a German
/// version, on a site with a default share image.
#[cfg(test)]
fn site_and_page() -> (SeoSite, SeoPage) {
    let en = Locale::try_new("en").unwrap();
    let address = AbsoluteUrl::parse("https://example.com/").unwrap();
    let mut site = SeoSite::new("Field Notes", address, en.clone());
    site.mount = UrlPath::try_new("/blog/").unwrap();
    site.languages.push(Locale::try_new("de").unwrap());
    let path = UrlPath::try_new("/links/").unwrap();
    let mut page = SeoPage::new(PageKind::Post, en, path, "Links & \"anchors\" <b>");
    page.excerpt = "Tom's guide to links.".to_owned();
    page.author = Some("Lena Vogt".to_owned());
    (site, page)
}

/// What a front end writes for `head`, escaping as the theme does.
#[cfg(test)]
fn printed(head: &PostHead) -> String {
    let escape = |text: &str| {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    };
    let mut out = String::new();
    for tag in head.tags() {
        let line = match tag {
            HeadElement::Title { text } => format!("<title>{}</title>", escape(text)),
            HeadElement::Meta {
                name,
                property,
                content,
            } => {
                let (attribute, value) = match (name, property) {
                    (Some(name), _) => ("name", name),
                    (None, Some(property)) => ("property", property),
                    (None, None) => panic!("a meta tag with neither name nor property"),
                };
                let content = escape(content);
                format!(
                    "<meta {attribute}=\"{}\" content=\"{content}\">",
                    escape(value)
                )
            }
            HeadElement::Link {
                rel,
                hreflang,
                href,
            } => {
                let language = hreflang
                    .as_deref()
                    .map(|code| format!(" hreflang=\"{}\"", escape(code)))
                    .unwrap_or_default();
                format!(
                    "<link rel=\"{}\"{language} href=\"{}\">",
                    escape(rel),
                    escape(href)
                )
            }
            other => panic!("a tag this test cannot print: {other:?}"),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("<script type=\"application/ld+json\">");
    out.push_str(head.json_ld());
    out.push_str("</script>\n");
    out
}

#[test]
fn printing_each_tag_then_the_graph_gives_the_theme_s_head_byte_for_byte() {
    let (site, page) = site_and_page();
    let tags = head_tags(&site, &page).unwrap();
    let graph = page_graph(&site, &page).unwrap();
    let head = PostHead::new(&tags, &graph);

    let theme = format!("{}{}", tags.to_html(), graph.to_script_tag());
    assert_eq!(printed(&head), theme);
}

/// The robots directives of an indexed page.
const ROBOTS: &str = "index, follow, max-snippet:-1, max-image-preview:large, max-video-preview:-1";

#[test]
fn the_head_is_ordered_tags_and_the_graph_as_text() {
    let (site, page) = site_and_page();
    let tags = head_tags(&site, &page).unwrap();
    let graph = page_graph(&site, &page).unwrap();
    let head = serde_json::to_value(PostHead::new(&tags, &graph)).unwrap();

    let first: Vec<&Value> = head["tags"].as_array().unwrap().iter().take(4).collect();
    assert_eq!(
        first,
        [
            &json!({"tag": "title", "text": "Links & \"anchors\" <b> | Field Notes"}),
            &json!({"tag": "meta", "name": "description", "content": "Tom's guide to links."}),
            &json!({"tag": "meta", "name": "robots", "content": ROBOTS}),
            &json!({"tag": "link", "rel": "canonical", "href": "https://example.com/blog/links/"}),
        ]
    );
    let og = head["tags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tag| tag["property"] == "og:title");
    assert_eq!(og.map(|tag| &tag["tag"]), Some(&json!("meta")));
    let json_ld = head["json_ld"].as_str().unwrap();
    assert!(!json_ld.contains('<'), "{json_ld}");
    let graph: Value = serde_json::from_str(json_ld).unwrap();
    assert_eq!(graph["@context"], "https://schema.org");
}
