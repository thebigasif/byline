//! A post places a snippet with `::snippet{slug="..."}`, and the page shows
//! the snippet's blocks as its file holds them, so one change shows in
//! every post (plan 4.6, KH-EDT-1).

use std::path::Path;

use khalid_content::{
    BlockError, BlockKind, ContentError, ContentTree, SiteContent, Wikilinks, check_bodies,
    parse_blocks, read_content, render, render_linked,
};
use khalid_core::{Config, Environment, Locale, Slug, SlugMode};
use khalid_routes::Routes;

const DEMO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-site");

const DISCLOSURE: &str = "Some links in this post are affiliate links.";

#[cfg(test)]
fn demo() -> (Config, SiteContent) {
    let site = Path::new(DEMO);
    let config = Config::load(site, &Environment::empty()).unwrap();
    let content = read_content(&ContentTree::walk(site).unwrap()).unwrap();
    (config, content)
}

#[cfg(test)]
fn english(config: &Config, content: &SiteContent) -> Wikilinks {
    let en = Locale::try_new("en").unwrap();
    Wikilinks::for_language(content, config, &Routes::default(), &en)
}

#[test]
fn a_snippet_block_names_its_slug() {
    let blocks = parse_blocks("::snippet{slug=\"affiliate-disclosure\"}").unwrap();
    let slug = "affiliate-disclosure".to_owned();
    assert_eq!(blocks[0].kind, BlockKind::Snippet { slug });
    let missing = parse_blocks("::snippet").unwrap_err();
    assert!(matches!(missing, BlockError::Missing { .. }), "{missing:?}");
    let wrong = parse_blocks("::snippet{slug=\"Not a slug\"}").unwrap_err();
    assert!(matches!(wrong, BlockError::Value { .. }), "{wrong:?}");
}

#[test]
fn the_page_shows_the_snippet_the_post_places() {
    let (config, content) = demo();
    let links = english(&config, &content);
    let body = "Before.\n\n::snippet{slug=\"affiliate-disclosure\"}\n\nAfter.";
    let html = render_linked(
        body,
        &Locale::try_new("en").unwrap(),
        config.slugs(),
        &links,
    )
    .unwrap()
    .html;
    let (before, rest) = html.split_once(DISCLOSURE).unwrap();
    assert!(before.contains("<p>Before.</p>"), "{html}");
    assert!(rest.contains("<p>After.</p>"), "{html}");
}

#[test]
fn a_change_to_the_snippet_shows_in_every_post() {
    let mut links = Wikilinks::new();
    let slug: Slug = "cta".parse().unwrap();
    let en = Locale::try_new("en").unwrap();
    let body = "::snippet{slug=\"cta\"}";
    links.insert_snippet(
        &slug,
        "## Join us\n\n:::callout{kind=\"tip\"}\nSign up.\n:::",
    );
    let first = render_linked(body, &en, SlugMode::default(), &links).unwrap();
    assert!(first.html.contains("<h2 id=\"join-us\">"), "{}", first.html);
    assert!(first.html.contains("callout-tip"), "{}", first.html);
    assert_eq!(first.headings[0].text, "Join us");
    let mut changed = Wikilinks::new();
    changed.insert_snippet(&slug, "Changed once.");
    let again = render_linked(body, &en, SlugMode::default(), &changed).unwrap();
    assert!(
        again.html.contains("<p>Changed once.</p>"),
        "{}",
        again.html
    );
}

#[test]
fn a_snippet_inside_a_snippet_and_an_unknown_one_show_nothing() {
    let mut links = Wikilinks::new();
    let en = Locale::try_new("en").unwrap();
    links.insert_snippet(
        &"loop".parse().unwrap(),
        "Once.\n\n::snippet{slug=\"loop\"}",
    );
    let html = render_linked("::snippet{slug=\"loop\"}", &en, SlugMode::default(), &links)
        .unwrap()
        .html;
    assert_eq!(html.matches("Once.").count(), 1, "{html}");
    let alone = render("::snippet{slug=\"loop\"}", &en, SlugMode::default()).unwrap();
    assert_eq!(alone.html.trim(), "");
}

#[test]
fn the_reader_refuses_a_snippet_the_language_does_not_have() {
    let (config, mut content) = demo();
    let post = content
        .posts
        .iter_mut()
        .find(|doc| doc.locale.as_str() == "de");
    let post = post.unwrap();
    post.body = format!(
        "Text.\n\n::snippet{{slug=\"affiliate-disclosure\"}}\n{}",
        post.body
    );
    let body_line = post.body_line;
    let error = check_bodies(&content, &config, &Routes::default()).unwrap_err();
    let ContentError::Snippet {
        line, slug, path, ..
    } = &error
    else {
        panic!("{error:?}");
    };
    assert_eq!(slug, "affiliate-disclosure");
    assert_eq!(*line, body_line + 2, "{}", path.display());
    let message = error.to_string();
    assert!(
        message.contains("no snippet called affiliate-disclosure in de"),
        "{message}"
    );
}
