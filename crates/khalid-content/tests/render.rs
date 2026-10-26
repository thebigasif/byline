//! Markdown to HTML: the standard blocks, escaped raw HTML, heading ids and
//! the facts templates need beside the HTML.

use khalid_content::render;
use khalid_core::{Locale, SlugMode};

#[cfg(test)]
fn en() -> Locale {
    Locale::try_new("en").unwrap()
}

#[test]
fn renders_the_standard_blocks() {
    let markdown = "\
## Why internal links matter

A paragraph with **bold**, *emphasis*, `code`, ~~old~~ and a [link](/blog/title-tags/).

> A quote from a reader.

- one
- two

1. first
2. second

- [x] done
- [ ] to do

| Page | Links in |
|---|---|
| Home | 12 |

```rust
let links = 3;
```

---

A claim that needs a source.[^1]

[^1]: The source.
";
    insta::assert_snapshot!(render(markdown, &en(), SlugMode::Ascii).html);
}

#[test]
fn raw_html_is_shown_as_text_not_run() {
    let html = render(
        "<script>alert(1)</script>\n\nText with <b>bold</b>.\n",
        &en(),
        SlugMode::Ascii,
    )
    .html;
    assert!(!html.contains("<script>"), "{html}");
    assert!(html.contains("&lt;script&gt;"), "{html}");
    assert!(!html.contains("<b>"), "{html}");
}

#[test]
fn headings_get_stable_ids_and_are_listed() {
    let rendered = render(
        "## Title tags\n\n### Title tags\n\n## Über Größe\n\n## ?!\n",
        &Locale::try_new("de").unwrap(),
        SlugMode::Ascii,
    );
    let headings: Vec<(u8, &str, &str)> = rendered
        .headings
        .iter()
        .map(|h| (h.level, h.id.as_str(), h.text.as_str()))
        .collect();
    assert_eq!(
        headings,
        [
            (2, "title-tags", "Title tags"),
            (3, "title-tags-2", "Title tags"),
            (2, "ueber-groesse", "Über Größe"),
            (2, "section", "?!"),
        ]
    );
    assert!(
        rendered
            .html
            .contains("<h3 id=\"title-tags-2\">Title tags</h3>"),
        "{}",
        rendered.html
    );
}

#[test]
fn counts_the_words_a_reader_reads() {
    let rendered = render(
        "## Two words\n\nThree more **words** here, `code` too.\n",
        &en(),
        SlugMode::Ascii,
    );
    assert_eq!(rendered.words, 8);
}

#[test]
fn a_unicode_site_keeps_accents_in_heading_ids() {
    let rendered = render(
        "## Über Größe\n",
        &Locale::try_new("de").unwrap(),
        SlugMode::Unicode,
    );
    assert_eq!(rendered.headings[0].id, "über-größe");
}

#[test]
fn lists_links_in_order_and_tells_internal_from_outside() {
    let rendered = render(
        "Read [title tags](/blog/title-tags/), the [guide](https://example.com/guide), \
         [this part](#why) and <https://serpdad.com>.\n",
        &en(),
        SlugMode::Ascii,
    );
    let links: Vec<(&str, &str, bool)> = rendered
        .links
        .iter()
        .map(|l| (l.href.as_str(), l.text.as_str(), l.internal))
        .collect();
    assert_eq!(
        links,
        [
            ("/blog/title-tags/", "title tags", true),
            ("https://example.com/guide", "guide", false),
            ("#why", "this part", true),
            ("https://serpdad.com", "https://serpdad.com", false),
        ]
    );
}

#[test]
fn lists_images_with_their_alt_text_or_none() {
    let rendered = render(
        "![A bakery shelf](shelf.jpg)\n\n![](chart.png)\n",
        &en(),
        SlugMode::Ascii,
    );
    let images: Vec<(&str, Option<&str>)> = rendered
        .images
        .iter()
        .map(|i| (i.src.as_str(), i.alt.as_deref()))
        .collect();
    assert_eq!(
        images,
        [("shelf.jpg", Some("A bakery shelf")), ("chart.png", None)]
    );
}
