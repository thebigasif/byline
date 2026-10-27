//! Reading the post fields for the lead image, the search title, tags and
//! the sitemap. Each field has a test for what it reads and one for the
//! exact message when it is wrong.

use std::path::Path;

use khalid_content::{Sitemap, parse_post};

#[cfg(test)]
fn message(text: &str) -> String {
    parse_post(Path::new("posts/a.md"), text)
        .unwrap_err()
        .to_string()
}

#[test]
fn reads_the_lead_image_with_its_alt_text_and_caption() {
    let text = "---\ntitle: A\nimage:\n  src: cover.jpg\n  alt: A map of internal links\n  caption: Links between five posts.\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    let image = post.front.image.unwrap();
    assert_eq!(image.src, "cover.jpg");
    assert_eq!(image.alt, "A map of internal links");
    assert_eq!(image.caption.as_deref(), Some("Links between five posts."));
    let plain = parse_post(Path::new("posts/a.md"), "---\ntitle: A\n---\n").unwrap();
    assert_eq!(plain.front.image, None);
}

#[test]
fn refuses_a_lead_image_without_alt_text() {
    assert_eq!(
        message("---\nimage:\n  src: cover.jpg\n  alt: \" \"\n---\n"),
        "posts/a.md, line 3, field \"image\": the image needs alt text that says what it shows, such as alt: A map of internal links"
    );
}

#[test]
fn refuses_a_lead_image_without_a_source() {
    assert_eq!(
        message("---\nimage:\n  alt: A map\n---\n"),
        "posts/a.md, line 3, field \"image\": missing field `src`"
    );
}

#[test]
fn reads_the_search_title_and_refuses_a_blank_one() {
    let text = "---\ntitle: A\nseo_title: Internal links, a guide for 2026\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    assert_eq!(
        post.front.seo_title.as_deref(),
        Some("Internal links, a guide for 2026")
    );
    assert_eq!(
        message("---\nseo_title: \"\"\n---\n"),
        "posts/a.md, line 2, field \"seo_title\": the search title needs at least one word; remove the field to use the post's title"
    );
}

#[test]
fn reads_tags_as_one_name_or_a_list() {
    let text = "---\ntitle: A\ntags:\n  - internal links\n  - Anchor text\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    assert_eq!(post.front.tags, ["internal links", "Anchor text"]);
    let one = parse_post(Path::new("posts/a.md"), "---\ntags: guides\n---\n").unwrap();
    assert_eq!(one.front.tags, ["guides"]);
    let plain = parse_post(Path::new("posts/a.md"), "---\ntitle: A\n---\n").unwrap();
    assert_eq!(plain.front.tags, Vec::<String>::new());
}

#[test]
fn refuses_a_blank_tag_and_a_tag_named_twice() {
    assert_eq!(
        message("---\ntags: [guides, \" \"]\n---\n"),
        "posts/a.md, line 2, field \"tags\": a tag needs at least one word, such as internal links; remove the empty one"
    );
    assert_eq!(
        message("---\ntags: [Guides, guides]\n---\n"),
        "posts/a.md, line 2, field \"tags\": the tag \"guides\" is listed twice; keep one"
    );
}

#[test]
fn a_post_is_in_the_sitemap_unless_it_says_sitemap_false() {
    let plain = parse_post(Path::new("posts/a.md"), "---\ntitle: A\n---\n").unwrap();
    assert_eq!(plain.front.sitemap, Sitemap::Listed);
    let empty = parse_post(Path::new("posts/a.md"), "---\n---\n").unwrap();
    assert_eq!(empty.front.sitemap, Sitemap::Listed);
    let left = parse_post(Path::new("posts/a.md"), "---\nsitemap: false\n---\n").unwrap();
    assert_eq!(left.front.sitemap, Sitemap::Unlisted);
    let listed = parse_post(Path::new("posts/a.md"), "---\nsitemap: true\n---\n").unwrap();
    assert_eq!(listed.front.sitemap, Sitemap::Listed);
}
