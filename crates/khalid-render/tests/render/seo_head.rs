//! A head from the SEO crate takes the place of the template's own tags.

use khalid_render::{HeadHtml, PageBody, PageKind, Renderer};

use crate::common::{demo_site, meta};

const SEO_HEAD: &str = "<title>Why links matter | Field Notes on SEO</title>\n\
<meta name=\"robots\" content=\"index, follow\">\n\
<script type=\"application/ld+json\">{\"@graph\":[]}</script>\n";

fn render_with(head: Option<HeadHtml>) -> String {
    let mut meta = meta(
        "Why links matter",
        "/blog/why-links-matter/",
        PageKind::Article,
    );
    meta.seo_head = head;
    Renderer::new()
        .unwrap()
        .render(&demo_site("en"), &meta, &PageBody::NotFound)
        .unwrap()
}

#[test]
fn a_seo_head_is_printed_as_it_is_in_place_of_the_template_tags() {
    let page = render_with(Some(HeadHtml::from_trusted(SEO_HEAD.to_owned())));
    let head = page.split("</head>").next().unwrap();
    assert!(head.contains(SEO_HEAD), "{head}");
    assert_eq!(head.matches("<title>").count(), 1, "{head}");
    assert!(!head.contains("og:title"), "{head}");
    assert!(!head.contains("rel=\"canonical\""), "{head}");
    assert!(
        head.contains("<link rel=\"stylesheet\" href=\"/blog/assets/site.0123abcd.css\">"),
        "{head}"
    );
    assert!(head.starts_with("<!doctype html>"), "{head}");
    assert!(head.contains("<meta charset=\"utf-8\">"), "{head}");
}

#[test]
fn without_a_seo_head_the_template_writes_its_own_tags() {
    let page = render_with(None);
    let head = page.split("</head>").next().unwrap();
    assert!(head.contains("<title>Why links matter</title>"), "{head}");
    assert!(head.contains("og:title"), "{head}");
}
