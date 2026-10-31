//! Reading the post fields for the lead image, the search title, tags, the
//! sitemap and the topic cluster. Each field has a test for what it reads
//! and one for the exact message when it is wrong.

use std::path::Path;

use khalid_content::{ReviewState, ShareCards, Sitemap, TopicRole, parse_post};
use khalid_core::Slug;

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
fn reads_co_authors_and_the_review_state() {
    let text = "---\nauthor: asif\nco_authors: [lena-vogt, mira]\nreview_state: in_review\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    let names: Vec<&str> = post.front.co_authors.iter().map(Slug::as_str).collect();
    assert_eq!(names, ["lena-vogt", "mira"]);
    assert_eq!(post.front.review_state, ReviewState::InReview);
    let plain = parse_post(Path::new("posts/a.md"), "---\n---\n").unwrap();
    assert_eq!(plain.front.review_state, ReviewState::Draft);
    assert_eq!(plain.front.co_authors, Vec::<Slug>::new());
}

#[test]
fn refuses_a_review_state_it_does_not_know() {
    assert_eq!(
        message("---\nreview_state: done\n---\n"),
        "posts/a.md, line 2, field \"review_state\": unknown variant `done`, expected one of draft, in_review, ready"
    );
}

#[test]
fn reads_share_cards_for_x_and_facebook() {
    let text = "---\nshare_cards:\n  x:\n    title: Internal links in five minutes\n  facebook:\n    description: A short guide.\n    image: card.png\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    let cards = post.front.share_cards;
    let x = cards.x.unwrap();
    assert_eq!(x.title.as_deref(), Some("Internal links in five minutes"));
    assert_eq!(x.description, None);
    let facebook = cards.facebook.unwrap();
    assert_eq!(facebook.description.as_deref(), Some("A short guide."));
    assert_eq!(facebook.image.as_deref(), Some("card.png"));
    let plain = parse_post(Path::new("posts/a.md"), "---\n---\n").unwrap();
    assert_eq!(plain.front.share_cards, ShareCards::default());
}

#[test]
fn refuses_a_share_card_for_a_network_it_does_not_know() {
    assert_eq!(
        message("---\nshare_cards:\n  mastodon:\n    title: A\n---\n"),
        "posts/a.md, line 3, field \"share_cards\": unknown field `mastodon`, expected one of x, facebook"
    );
}

#[test]
fn reads_hand_written_schema_as_text_and_refuses_a_blank_one() {
    let text = "---\nschema: |\n  {\"@type\": \"Event\", \"name\": \"Links clinic\"}\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    assert_eq!(
        post.front.schema.as_deref(),
        Some("{\"@type\": \"Event\", \"name\": \"Links clinic\"}\n")
    );
    assert_eq!(
        message("---\nschema: \" \"\n---\n"),
        "posts/a.md, line 2, field \"schema\": the schema field needs JSON-LD, such as {\"@type\": \"Event\"}; remove the field if the post has none"
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

#[test]
fn reads_a_topic_cluster_whose_role_is_member_unless_it_says_pillar() {
    let member = "---\ntitle: A\ncluster:\n  name: internal-links\n---\n";
    let post = parse_post(Path::new("posts/a.md"), member).unwrap();
    let cluster = post.front.cluster.unwrap();
    assert_eq!(cluster.name.as_str(), "internal-links");
    assert_eq!(cluster.role, TopicRole::Member);
    let pillar = "---\ncluster:\n  name: internal-links\n  role: pillar\n---\n";
    let post = parse_post(Path::new("posts/a.md"), pillar).unwrap();
    assert_eq!(post.front.cluster.unwrap().role, TopicRole::Pillar);
    let plain = parse_post(Path::new("posts/a.md"), "---\ntitle: A\n---\n").unwrap();
    assert_eq!(plain.front.cluster, None);
}

#[test]
fn refuses_a_cluster_without_a_name_or_with_an_unknown_role() {
    assert_eq!(
        message("---\ncluster:\n  role: pillar\n---\n"),
        "posts/a.md, line 3, field \"cluster\": missing field `name`"
    );
    assert_eq!(
        message("---\ncluster:\n  name: internal-links\n  role: leader\n---\n"),
        "posts/a.md, line 4, field \"cluster\": unknown variant `leader`, expected one of pillar, member"
    );
}
