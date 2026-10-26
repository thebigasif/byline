//! Wikilinks: `[[slug]]` becomes a link to the post, page or glossary term
//! with that slug in the same language.

use khalid_content::{WikiTarget, Wikilinks, render, render_linked};
use khalid_core::{Locale, Slug, SlugMode};

#[cfg(test)]
fn en() -> Locale {
    Locale::try_new("en").unwrap()
}

#[cfg(test)]
fn targets() -> Wikilinks {
    let mut links = Wikilinks::new();
    links.insert(
        &Slug::try_new("topical-authority").unwrap(),
        WikiTarget::new("/blog/glossary/topical-authority/", "Topical authority"),
    );
    links.insert(
        &Slug::try_new("ab-tests").unwrap(),
        WikiTarget::new("/blog/ab-tests/", "A <b> & \"B\" tests"),
    );
    links
}

#[cfg(test)]
fn html(markdown: &str) -> String {
    render_linked(markdown, &en(), SlugMode::Ascii, &targets())
        .unwrap()
        .html
}

#[test]
fn a_wikilink_becomes_a_link_named_after_its_target() {
    assert_eq!(
        html("That is how you build [[topical-authority]].\n"),
        "<p>That is how you build <a href=\"/blog/glossary/topical-authority/\">Topical authority</a>.</p>\n"
    );
}

#[test]
fn text_after_a_pipe_is_the_link_text() {
    assert_eq!(
        html("Read [[topical-authority|what authority means]].\n"),
        "<p>Read <a href=\"/blog/glossary/topical-authority/\">what authority means</a>.</p>\n"
    );
}

#[test]
fn a_target_title_is_escaped() {
    assert_eq!(
        html("See [[ab-tests]].\n"),
        "<p>See <a href=\"/blog/ab-tests/\">A &lt;b&gt; &amp; &quot;B&quot; tests</a>.</p>\n"
    );
}

#[test]
fn a_wikilink_is_an_internal_link_for_the_linter() {
    let rendered = render_linked(
        "That is [[topical-authority]].\n",
        &en(),
        SlugMode::Ascii,
        &targets(),
    )
    .unwrap();
    let link = rendered.links.first().unwrap();
    assert_eq!(link.href, "/blog/glossary/topical-authority/");
    assert_eq!(link.text, "Topical authority");
    assert!(link.internal);
}

#[test]
fn a_wikilink_resolves_inside_a_directive() {
    let html = html(":::callout{kind=\"tip\"}\nSee [[topical-authority]].\n:::\n");
    assert!(
        html.contains("<a href=\"/blog/glossary/topical-authority/\">Topical authority</a>"),
        "{html}"
    );
}

#[test]
fn an_unknown_target_shows_its_text_without_a_link() {
    assert_eq!(
        html("See [[no-such-post]] and [[nothing|this]].\n"),
        "<p>See no-such-post and this.</p>\n"
    );
}

#[test]
fn a_wikilink_in_code_stays_code() {
    assert_eq!(
        html("Write `[[topical-authority]]` to link.\n"),
        "<p>Write <code>[[topical-authority]]</code> to link.</p>\n"
    );
}

#[test]
fn render_without_targets_links_nothing() {
    let rendered = render("See [[topical-authority]].\n", &en(), SlugMode::Ascii).unwrap();
    assert_eq!(rendered.html, "<p>See topical-authority.</p>\n");
    assert_eq!(rendered.links.len(), 0, "{:?}", rendered.links);
}
