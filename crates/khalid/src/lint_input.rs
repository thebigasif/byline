//! Turns the content files into the linter's own model (ADR 0011).
//!
//! The lint crate never reads files, so this binary maps parsed posts and
//! their rendered output to `LintPost` and `LintSite`. Wikilinks (task 1.8),
//! post paths (task 1.9) and the home page's links are not known yet, so the
//! rules that need them use their fallbacks or skip.

use std::path::Path;

use chrono::NaiveDate;
use khalid_content::{Document, PostFront, SiteContent, render};
use khalid_core::{Config, Locale, Slug};
use khalid_lint::{
    GlossaryTerm, Heading, HeadingLevel, Image, LinkKind, LintLink, LintPost, LintSite, PageRef,
};

/// The site's posts, pages and glossary in the lint model, and the document
/// each post came from, in the same order.
pub(crate) fn lint_site<'c>(
    content: &'c SiteContent,
    config: &Config,
) -> (LintSite, Vec<&'c Document<PostFront>>) {
    let mut site = LintSite::default();
    let mut documents = Vec::new();
    for document in &content.posts {
        if let Some(post) = lint_post(document, config) {
            site.posts.push(post);
            documents.push(document);
        }
    }
    for page in &content.pages {
        if let Some(slug) = slug_of(page.front.slug.as_ref(), &page.path, &page.locale, config) {
            site.pages.push(PageRef::new(page.locale.clone(), slug));
        }
    }
    for entry in &content.glossary {
        let slug = slug_of(
            entry.front.slug.as_ref(),
            &entry.path,
            &entry.locale,
            config,
        );
        if let (Some(term), Some(slug)) = (&entry.front.term, slug) {
            site.glossary
                .push(GlossaryTerm::new(entry.locale.clone(), term, slug));
        }
    }
    site.locales = config.languages().to_vec();
    site.slugs = Some(config.slugs());
    (site, documents)
}

/// One post in the lint model, or `None` when no slug can be made for it or
/// its body does not render. Reading the site already refuses a broken
/// block, so a post that got this far renders.
fn lint_post(document: &Document<PostFront>, config: &Config) -> Option<LintPost> {
    let parts = PostParts {
        locale: &document.locale,
        path: &document.path,
        front: &document.front,
        body: &document.body,
    };
    lint_parts(&parts, config)
}

/// A post that may not be a file under `content/` yet, such as a draft
/// about to be published: its language, where it is or will be, its front
/// matter and its markdown body.
pub(crate) struct PostParts<'a> {
    pub(crate) locale: &'a Locale,
    pub(crate) path: &'a Path,
    pub(crate) front: &'a PostFront,
    pub(crate) body: &'a str,
}

/// The post made of `parts` in the lint model, or `None` as for
/// [`lint_post`].
pub(crate) fn lint_parts(parts: &PostParts<'_>, config: &Config) -> Option<LintPost> {
    let front = parts.front;
    let slug = slug_of(front.slug.as_ref(), parts.path, parts.locale, config)?;
    let rendered = render(parts.body, parts.locale, config.slugs()).ok()?;
    let mut post = LintPost::new(parts.locale.clone(), slug);
    post.title.clone_from(&front.title);
    post.description.clone_from(&front.description);
    post.date = front.date;
    post.author = front
        .author
        .as_ref()
        .map(|author| author.as_str().to_owned());
    post.translation.clone_from(&front.translation);
    post.headings = rendered
        .headings
        .iter()
        .filter_map(|h| level(h.level).map(|level| Heading::new(level, h.text.clone())))
        .collect();
    post.links = rendered
        .links
        .iter()
        .map(|link| {
            let kind = if link.internal {
                LinkKind::Internal
            } else {
                LinkKind::External
            };
            LintLink::new(kind, link.href.clone(), link.text.clone())
        })
        .collect();
    post.images = rendered
        .images
        .iter()
        .map(|image| Image::new(image.src.clone(), image.alt.clone()))
        .collect();
    post.word_count = rendered.words;
    post.body_text = plain_text(&rendered.html);
    Some(post)
}

/// The slug from the header, or else from the file name: `index.md` takes
/// its folder's name, and a leading `2026-09-14-` date is dropped.
fn slug_of(front: Option<&Slug>, path: &Path, locale: &Locale, config: &Config) -> Option<Slug> {
    if let Some(slug) = front {
        return Some(slug.clone());
    }
    let stem = path.file_stem()?.to_str()?;
    let stem = if stem == "index" {
        path.parent()?.file_name()?.to_str()?
    } else {
        stem
    };
    let undated = match (stem.get(..10), stem.get(10..11), stem.get(11..)) {
        (Some(day), Some("-"), Some(rest))
            if NaiveDate::parse_from_str(day, "%Y-%m-%d").is_ok() =>
        {
            rest
        }
        _ => stem,
    };
    Slug::from_title(undated, locale, config.slugs()).ok()
}

/// The lint model's heading level for an HTML level from 1 to 6.
fn level(number: u8) -> Option<HeadingLevel> {
    match number {
        1 => Some(HeadingLevel::H1),
        2 => Some(HeadingLevel::H2),
        3 => Some(HeadingLevel::H3),
        4 => Some(HeadingLevel::H4),
        5 => Some(HeadingLevel::H5),
        6 => Some(HeadingLevel::H6),
        _ => None,
    }
}

/// The words of rendered HTML with every tag and every entity, such as
/// `&quot;`, replaced by a space, so neither is read as a word.
fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut skip_until = None;
    for c in html.chars() {
        match (skip_until, c) {
            (Some(end), _) if c == end => {
                skip_until = None;
                text.push(' ');
            }
            (Some(_), _) => {}
            (None, '<') => skip_until = Some('>'),
            (None, '&') => skip_until = Some(';'),
            (None, _) => text.push(c),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_drops_tags_and_entities() {
        let html = "<p>Write a <em>&quot;title&quot;</em> tag &amp; test it.</p>";
        let text = plain_text(html);
        let words: Vec<&str> = text.split_whitespace().collect();
        assert_eq!(words, ["Write", "a", "title", "tag", "test", "it."]);
    }
}
