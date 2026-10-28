//! A Medium body as Khalid markdown. Medium repeats the headline and the
//! subtitle at the top of the body, wraps the rest in sections, writes
//! code with `<br>` for its line breaks, and shows images, frames and link
//! cards inside its own markup; each becomes what Khalid writes for it.

use std::io;

use khalid_content::BlockError;
use khalid_core::{AbsoluteUrl, Locale, SlugMode};
use percent_encoding::percent_decode_str;
use scraper::{ElementRef, Html as Page, Node};

use crate::find::{first, has_class, select, words};
use crate::markdown::is_safe;
use crate::pieces::{Converted, Html, converted};

/// Why a Medium body could not be made into markdown.
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

/// The markdown of a post's `body`, the HTML Medium wrote, for a site
/// written in `locale` with slugs in `slugs`. A frame or image whose
/// address cannot be kept is left out and named in [`Converted::left`].
pub(crate) fn medium_body(
    body: &str,
    locale: &Locale,
    slugs: SlugMode,
) -> Result<Converted, BodyError> {
    let page = Page::parse_fragment(body);
    let sections = select(page.root_element(), "section.section");
    let mut html = Html::default();
    if sections.is_empty() {
        html.tag(body);
    }
    for (index, section) in sections.into_iter().enumerate() {
        if index > 0 {
            // Medium shows a break between sections.
            html.tag("<hr>\n\n");
        }
        for inner in select(section, ".section-inner") {
            for graf in inner.children().filter_map(ElementRef::wrap) {
                let copy = has_class(graf, "graf--title") || has_class(graf, "graf--subtitle");
                if index == 0 && copy {
                    continue;
                }
                write_graf(graf, &mut html);
                html.tag("\n\n");
            }
        }
    }
    converted(html, locale, slugs)
}

/// Writes one of Medium's paragraphs, headings, code, figures or cards.
/// Medium has two heading sizes, `h3` and `h4`, under the headline, so
/// they become `h2` and `h3`.
fn write_graf(graf: ElementRef<'_>, html: &mut Html) {
    match graf.value().name() {
        "h3" => html.tag(&format!("<h2>{}</h2>", graf.inner_html())),
        "h4" => html.tag(&format!("<h3>{}</h3>", graf.inner_html())),
        "pre" => {
            let mut code = String::new();
            code_text(graf, &mut code);
            let language = graf.value().attr("data-code-block-lang").unwrap_or("");
            html.code(language, &code);
        }
        "figure" => figure(graf, html),
        "div" if has_class(graf, "graf--mixtapeEmbed") => link_card(graf, html),
        _ => html.tag(&graf.html()),
    }
}

/// The text of code, with each `<br>` as a line break.
fn code_text(element: ElementRef<'_>, code: &mut String) {
    for child in element.children() {
        match child.value() {
            Node::Text(text) => code.push_str(text),
            Node::Element(inner) if inner.name() == "br" => code.push('\n'),
            _ => {
                if let Some(inner) = ElementRef::wrap(child) {
                    code_text(inner, code);
                }
            }
        }
    }
}

/// An image as a `figure` block with its caption, or a frame as an
/// `embed` block with its caption after it.
fn figure(figure: ElementRef<'_>, html: &mut Html) {
    let caption = first(figure, "figcaption").map(|caption| caption.inner_html());
    if let Some(image) = first(figure, "img") {
        let src = image.value().attr("src").filter(|src| is_safe(src));
        match src {
            Some(src) => {
                let alt = image.value().attr("alt").unwrap_or("");
                let body = caption.unwrap_or_default();
                html.block("figure", &[("src", src), ("alt", alt)], Some(&body));
            }
            None => html.leave("image"),
        }
    } else if let Some(frame) = first(figure, "iframe") {
        match frame_address(frame) {
            Some(src) => {
                // A screen reader names the player by its title; Medium's
                // frames rarely have one, so the caption comes next.
                let caption_words = first(figure, "figcaption").map(words);
                let title = frame
                    .value()
                    .attr("title")
                    .map(str::to_owned)
                    .or(caption_words)
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| src.host().to_owned());
                html.block("embed", &[("src", src.as_str()), ("title", &title)], None);
                if let Some(caption) = caption {
                    html.tag(&format!("<p>{caption}</p>"));
                }
            }
            None => html.leave("frame"),
        }
    } else {
        html.tag(&figure.html());
    }
}

/// The page a frame shows. Medium serves most frames through Embedly,
/// whose address carries the page as its `url` or `src` value. A frame
/// with a path only, as Medium writes for a GitHub gist, has none.
fn frame_address(frame: ElementRef<'_>) -> Option<AbsoluteUrl> {
    let src = AbsoluteUrl::parse(frame.value().attr("src")?).ok()?;
    if src.host() != "cdn.embedly.com" {
        return Some(src);
    }
    let query = src.as_str().split_once('?')?.1;
    let value = |name: &str| {
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
            .map(|value| percent_decode_str(value).decode_utf8_lossy().into_owned())
    };
    let page = value("url").or_else(|| value("src"))?;
    AbsoluteUrl::parse(&page).ok()
}

/// A link card as a `bookmark` block with its title and description.
fn link_card(card: ElementRef<'_>, html: &mut Html) {
    let link = first(card, "a[href]");
    let href = link.and_then(|link| AbsoluteUrl::parse(link.value().attr("href")?).ok());
    let Some(href) = href else {
        html.tag(&card.html());
        return;
    };
    let text = |css: &str| first(card, css).map(words).filter(|text| !text.is_empty());
    let mut attributes = vec![("href", href.as_str().to_owned())];
    attributes.extend(text("strong").map(|title| ("title", title)));
    attributes.extend(text("em").map(|about| ("description", about)));
    let attributes: Vec<(&str, &str)> = attributes
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    html.block("bookmark", &attributes, None);
}

#[cfg(test)]
mod tests {
    use khalid_content::{BlockKind, parse_blocks};

    use super::*;

    /// Medium's sections around `grafs`, one string per section.
    fn sections(grafs: &[&str]) -> String {
        let open = r#"<section name="a1" class="section section--body"><div class="section-divider"><hr class="section-divider"></div><div class="section-content"><div class="section-inner sectionLayout--insetColumn">"#;
        let close = "</div></div></section>";
        grafs
            .iter()
            .flat_map(|inner| [open, inner, close])
            .collect()
    }

    fn markdown(grafs: &[&str]) -> Converted {
        let locale = Locale::try_new("en").unwrap();
        let body = medium_body(&sections(grafs), &locale, SlugMode::Ascii).unwrap();
        parse_blocks(&body.markdown).unwrap();
        body
    }

    #[test]
    fn the_repeated_headline_goes_and_headings_step_down() {
        let body = markdown(&[
            r#"<h3 class="graf graf--h3 graf--leading graf--title">Rye bread</h3><h4 class="graf graf--h4 graf--subtitle">A wetter dough</h4><p class="graf graf--p">Rye needs <strong>time</strong>.</p><h3 class="graf graf--h3">Mixing</h3><h4 class="graf graf--h4">By hand</h4>"#,
            r#"<h3 class="graf graf--h3 graf--title">Baking</h3><p class="graf graf--p">Bake it hot.</p>"#,
        ]);
        assert_eq!(
            body.markdown,
            "Rye needs **time**.\n\n## Mixing\n\n### By hand\n\n* * *\n\n## Baking\n\nBake it hot.\n",
            "only the first section repeats the headline; a later section starts after a break"
        );
        assert_eq!(body.left, Vec::<String>::new());
    }

    #[test]
    fn code_keeps_its_lines_and_language() {
        let body = markdown(&[
            r#"<pre class="graf graf--pre" data-code-block-lang="python">def feed(starter):<br>    starter.flour += 50<br><br>    return starter &lt; 1</pre>"#,
        ]);
        assert_eq!(
            body.markdown,
            "```python\ndef feed(starter):\n    starter.flour += 50\n\n    return starter < 1\n```\n"
        );
    }

    #[test]
    fn images_and_frames_become_blocks() {
        let body = markdown(&[concat!(
            r#"<figure class="graf graf--figure"><img class="graf-image" data-image-id="1*rye.jpeg" src="https://cdn-images-1.medium.com/max/800/1*rye.jpeg"><figcaption class="imageCaption">A <em>dark</em> loaf</figcaption></figure>"#,
            r#"<figure class="graf graf--iframe"><iframe src="https://cdn.embedly.com/widgets/media.html?src=https%3A%2F%2Fwww.youtube.com%2Fembed%2FdQw4w9WgXcQ&amp;url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DdQw4w9WgXcQ&amp;type=text%2Fhtml" width="640"></iframe><figcaption>Shaping a boule</figcaption></figure>"#,
            r#"<figure class="graf graf--iframe"><iframe src="/media/2b5f0c9e1d" width="700"></iframe></figure>"#,
        )]);
        assert_eq!(
            body.markdown,
            concat!(
                ":::figure{src=\"https://cdn-images-1.medium.com/max/800/1*rye.jpeg\" alt=\"\"}\nA *dark* loaf\n:::\n\n",
                "::embed{src=\"https://www.youtube.com/watch?v=dQw4w9WgXcQ\" title=\"Shaping a boule\"}\n\n",
                "Shaping a boule\n",
            )
        );
        assert_eq!(body.left, ["frame"], "a gist's frame names no page");
    }

    #[test]
    fn a_link_card_becomes_a_bookmark() {
        let body = markdown(&[
            r#"<div class="graf graf--mixtapeEmbed"><a href="https://northfield.example.com/rye/" class="markup--anchor markup--mixtapeEmbed-anchor"><strong class="markup--strong">Rye at home</strong><br><em class="markup--em">Our first loaf</em>northfield.example.com</a><a href="https://northfield.example.com/rye/" class="js-mixtapeImage mixtapeImage"></a></div>"#,
        ]);
        let blocks = parse_blocks(&body.markdown).unwrap();
        assert!(
            matches!(blocks[0].kind, BlockKind::Bookmark { .. }),
            "{blocks:?}"
        );
        assert_eq!(
            body.markdown,
            "::bookmark{href=\"https://northfield.example.com/rye/\" title=\"Rye at home\" description=\"Our first loaf\"}\n"
        );
    }

    #[test]
    fn scripts_and_unsafe_links_are_cleaned_out() {
        let body = markdown(&[
            r#"<p class="graf graf--p"><script>steal()</script><a href="javascript:steal()">Starter tips</a></p>"#,
        ]);
        assert_eq!(body.markdown, "Starter tips\n");
    }
}
