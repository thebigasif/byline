//! A Substack body as Khalid markdown. Substack writes plain paragraphs
//! and headings, and wraps images, videos, buttons and embedded posts in
//! its own markup; each becomes what Khalid writes for it, and a widget
//! with no place in Khalid, such as the subscribe box, is named and left
//! out.

use std::io;

use khalid_content::BlockError;
use khalid_core::{AbsoluteUrl, Locale, SlugMode};
use scraper::{ElementRef, Html as Page};

use crate::find::{first, has_class, select, words};
use crate::markdown::is_safe;
use crate::pieces::{Converted, Html, converted};

/// Why a Substack body could not be made into markdown.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BodyError {
    /// HTML in the body could not be read.
    #[error("its body could not be read as HTML")]
    Html(#[from] io::Error),
    /// The body made a block Khalid cannot read.
    #[error("its body holds a block Khalid cannot read")]
    Markdown(#[from] BlockError),
}

/// Substack's widgets with no place in Khalid, by class, each with the
/// name the report gives it.
const LEFT: &[(&str, &str)] = &[
    ("subscription-widget-wrap", "subscribe box"),
    ("subscription-widget-wrap-editor", "subscribe box"),
    ("button-wrapper", "button"),
    ("poll-embed", "poll"),
    ("tweet", "tweet"),
    ("install-substack-app-embed", "app box"),
    ("digest-post-embed", "post digest"),
    ("community-chat", "chat"),
    ("paywall-jump", "paywall line"),
];

/// The markdown of a post's `body`, the HTML Substack wrote, for a site
/// written in `locale` with slugs in `slugs`. Widgets Khalid has no place
/// for are named in [`Converted::left`].
pub(crate) fn substack_body(
    body: &str,
    locale: &Locale,
    slugs: SlugMode,
) -> Result<Converted, BodyError> {
    let page = Page::parse_fragment(body);
    let mut html = Html::default();
    for child in page.root_element().children() {
        match ElementRef::wrap(child) {
            Some(element) => write_element(element, &mut html),
            None => {
                if let Some(text) = child.value().as_text() {
                    html.escaped(text);
                }
            }
        }
        html.tag("\n\n");
    }
    converted(html, locale, slugs)
}

/// Writes one top-level element: an image, a video, an embedded post, a
/// widget left out, or anything else as it is.
fn write_element(element: ElementRef<'_>, html: &mut Html) {
    if let Some((_, kind)) = LEFT.iter().find(|(class, _)| has_class(element, class)) {
        html.leave(kind);
    } else if has_class(element, "captioned-image-container") || element.value().name() == "figure"
    {
        image(element, html);
    } else if has_class(element, "youtube-wrap") || has_class(element, "vimeo-wrap") {
        video(element, html);
    } else if has_class(element, "embedded-post-wrap") {
        embedded_post(element, html);
    } else if has_class(element, "pullquote") {
        html.tag(&format!(
            "<blockquote>{}</blockquote>",
            element.inner_html()
        ));
    } else {
        html.tag(&element.html());
    }
}

/// An image as a `figure` block with its caption.
fn image(container: ElementRef<'_>, html: &mut Html) {
    let src = first(container, "img").and_then(|image| {
        let src = image.value().attr("src").filter(|src| is_safe(src))?;
        Some((src, image.value().attr("alt").unwrap_or("")))
    });
    match src {
        Some((src, alt)) => {
            let caption = first(container, "figcaption").map(|caption| caption.inner_html());
            let body = caption.unwrap_or_default();
            html.block("figure", &[("src", src), ("alt", alt)], Some(&body));
        }
        None => html.leave("image"),
    }
}

/// A `YouTube` or Vimeo player as an `embed` block of the video's page.
fn video(wrap: ElementRef<'_>, html: &mut Html) {
    let src = first(wrap, "iframe")
        .and_then(|frame| frame.value().attr("src"))
        .and_then(|src| AbsoluteUrl::parse(src).ok());
    match src {
        Some(src) => {
            let title = first(wrap, "iframe")
                .and_then(|frame| frame.value().attr("title"))
                .filter(|title| !title.trim().is_empty())
                .unwrap_or(src.host())
                .to_owned();
            html.block("embed", &[("src", src.as_str()), ("title", &title)], None);
        }
        None => html.leave("video"),
    }
}

/// Another Substack post shown as a card, as a `bookmark` block.
fn embedded_post(wrap: ElementRef<'_>, html: &mut Html) {
    let href = select(wrap, "a[href]")
        .into_iter()
        .find_map(|link| AbsoluteUrl::parse(link.value().attr("href")?).ok());
    let Some(href) = href else {
        html.leave("embedded post");
        return;
    };
    let title = [".embedded-post-title", "h3"]
        .into_iter()
        .find_map(|css| first(wrap, css).map(words))
        .filter(|title| !title.is_empty());
    let mut attributes = vec![("href", href.as_str())];
    if let Some(title) = &title {
        attributes.push(("title", title));
    }
    html.block("bookmark", &attributes, None);
}

#[cfg(test)]
mod tests {
    use khalid_content::parse_blocks;

    use super::*;

    fn markdown(body: &str) -> Converted {
        let locale = Locale::try_new("en").unwrap();
        let converted = substack_body(body, &locale, SlugMode::Ascii).unwrap();
        parse_blocks(&converted.markdown).unwrap();
        converted
    }

    #[test]
    fn paragraphs_headings_and_pull_quotes_stay_as_written() {
        let body = markdown(
            r#"<p>Rye needs <strong>time</strong>.</p><h2>Mixing</h2><div class="pullquote"><p>Wait for it.</p></div><pre><code>feed(starter)</code></pre>"#,
        );
        assert_eq!(
            body.markdown,
            "Rye needs **time**.\n\n## Mixing\n\n> Wait for it.\n\n```\nfeed(starter)\n```\n"
        );
        assert!(body.left.is_empty(), "{:?}", body.left);
    }

    #[test]
    fn images_videos_and_embedded_posts_become_blocks() {
        let body = markdown(concat!(
            r#"<div class="captioned-image-container"><figure><a class="image-link" href="https://substackcdn.com/image/fetch/rye.jpg"><div class="image2-inset"><picture><img src="https://substackcdn.com/image/fetch/rye.jpg" alt="A rye loaf"></picture></div></a><figcaption class="image-caption">Out of the oven</figcaption></figure></div>"#,
            r#"<div id="youtube2-abc123" class="youtube-wrap" data-attrs="{}"><div class="youtube-inner"><iframe src="https://www.youtube-nocookie.com/embed/abc123?rel=0" title="Shaping rye"></iframe></div></div>"#,
            r#"<div class="embedded-post-wrap"><div class="embedded-post"><a href="https://rye.substack.com/p/spring-flour"><div class="embedded-post-title">Spring flour</div></a></div></div>"#,
        ));
        assert_eq!(
            body.markdown,
            ":::figure{src=\"https://substackcdn.com/image/fetch/rye.jpg\" alt=\"A rye loaf\"}\nOut of the oven\n:::\n\n\
             ::embed{src=\"https://www.youtube-nocookie.com/embed/abc123?rel=0\" title=\"Shaping rye\"}\n\n\
             ::bookmark{href=\"https://rye.substack.com/p/spring-flour\" title=\"Spring flour\"}\n"
        );
    }

    #[test]
    fn widgets_with_no_place_are_named_and_left_out() {
        let body = markdown(concat!(
            r#"<p>Before.</p><div class="subscription-widget-wrap"><form><input type="email"></form></div>"#,
            r#"<p class="button-wrapper"><a class="button primary" href="https://rye.substack.com/subscribe"><span>Subscribe now</span></a></p>"#,
            r#"<div class="captioned-image-container"><figure><img src="javascript:alert(1)"></figure></div><p>After.</p>"#,
        ));
        assert_eq!(body.markdown, "Before.\n\nAfter.\n");
        assert_eq!(body.left, ["subscribe box", "button", "image"]);
    }
}
