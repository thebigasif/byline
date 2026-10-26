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
    insta::assert_snapshot!(render(markdown, &en(), SlugMode::Ascii).unwrap().html);
}

#[test]
fn raw_html_is_shown_as_text_not_run() {
    let html = render(
        "<script>alert(1)</script>\n\nText with <b>bold</b>.\n",
        &en(),
        SlugMode::Ascii,
    )
    .unwrap()
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
    )
    .unwrap();
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
    )
    .unwrap();
    assert_eq!(rendered.words, 8);
}

#[test]
fn a_unicode_site_keeps_accents_in_heading_ids() {
    let rendered = render(
        "## Über Größe\n",
        &Locale::try_new("de").unwrap(),
        SlugMode::Unicode,
    )
    .unwrap();
    assert_eq!(rendered.headings[0].id, "über-größe");
}

#[test]
fn lists_links_in_order_and_tells_internal_from_outside() {
    let rendered = render(
        "Read [title tags](/blog/title-tags/), the [guide](https://example.com/guide), \
         [this part](#why) and <https://serpdad.com>.\n",
        &en(),
        SlugMode::Ascii,
    )
    .unwrap();
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
    )
    .unwrap();
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

#[test]
fn code_is_coloured_with_classes_not_inline_styles() {
    let html = render("```rust\nlet links = 3;\n```\n", &en(), SlugMode::Ascii)
        .unwrap()
        .html;
    assert!(html.contains("class=\"hl-"), "{html}");
    assert!(!html.contains("style="), "{html}");
}

#[test]
fn code_in_an_unknown_language_stays_plain_and_escaped() {
    let html = render(
        "```klingon\n<script>qapla</script>\n```\n",
        &en(),
        SlugMode::Ascii,
    )
    .unwrap()
    .html;
    assert!(html.contains("&lt;script&gt;"), "{html}");
    assert!(!html.contains("<script>"), "{html}");
}

#[test]
fn renders_callouts_and_figures() {
    let markdown = "\
## Before you start

:::callout{kind=tip}
Link from new posts to old ones, then **back again**.
:::

:::figure{src=\"shelf.jpg\" alt=\"A bakery shelf of sourdough loaves\"}
The shelf on a *Saturday* morning.
:::

That is all.
";
    insta::assert_snapshot!(render(markdown, &en(), SlugMode::Ascii).unwrap().html);
}

#[test]
fn a_figure_counts_as_an_image_with_its_alt_text() {
    let rendered = render(
        ":::figure{src=\"shelf.jpg\" alt=\"A shelf\"}\nCaption.\n:::\n",
        &en(),
        SlugMode::Ascii,
    )
    .unwrap();
    assert_eq!(rendered.images.len(), 1);
    assert_eq!(rendered.images[0].src, "shelf.jpg");
    assert_eq!(rendered.images[0].alt.as_deref(), Some("A shelf"));
}

#[test]
fn heading_ids_stay_unique_across_a_callout() {
    let rendered = render(
        "## Tips\n\n:::callout{kind=note}\n## Tips\n:::\n\n## Tips\n",
        &en(),
        SlugMode::Ascii,
    )
    .unwrap();
    let ids: Vec<&str> = rendered.headings.iter().map(|h| h.id.as_str()).collect();
    assert_eq!(ids, ["tips", "tips-2", "tips-3"]);
}

#[test]
fn figure_attributes_cannot_break_out_of_the_tag() {
    let html = render(
        ":::figure{src=\"a.jpg\" alt=\"Fish & chips <b>\"}\nCaption.\n:::\n",
        &en(),
        SlugMode::Ascii,
    )
    .unwrap()
    .html;
    assert!(
        html.contains("alt=\"Fish &amp; chips &lt;b&gt;\""),
        "{html}"
    );
}

#[test]
fn a_broken_directive_is_refused_not_rendered() {
    let error = render(
        ":::callout{kind=danger}\nCareful.\n:::\n",
        &en(),
        SlugMode::Ascii,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "line 1: a callout's kind is note, tip or warning, not danger"
    );
}
