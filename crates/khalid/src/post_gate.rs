//! Gated content on a running site (task 5.11). The public page of a post
//! with a `:::gated` block shows its note and a sign-up form, never the
//! body. Each such post also keeps a full page, its gated parts open,
//! which `khalid serve` sends only to a reader with the reader cookie (ADR
//! "gated content opens with a signed reader cookie").

use khalid_content::{Gated, SiteContent, Wikilinks, render_gated};
use khalid_core::{Config, UserMessage};
use khalid_render::{BodyHtml, PageBody, ReadingTime};
use khalid_routes::Routes;

use crate::build_pages::LanguagePages;
use crate::post_lock::{Key, Lock, Unlocked};
use crate::{build_message, build_seo};

/// How the renderer starts a locked gated part, and an open one.
const LOCKED: &str = "<section class=\"gated\">\n";
const OPEN: &str = "<section class=\"gated gated-open\">\n";

/// How every gated part ends. Neither holds a section of its own.
const END: &str = "</section>\n";

/// Keeps a full page for each post of `content` in `languages` with gated
/// content: a copy of its public page, finished as the public page is,
/// with each locked part swapped for the part open. A post with a password
/// or for members is skipped: its body stays locked as a whole.
///
/// # Errors
///
/// A [`UserMessage`] when a post's body cannot be rendered, which the
/// public page's render rules out.
pub(crate) fn open(
    config: &Config,
    routes: &Routes,
    content: &SiteContent,
    languages: &mut [LanguagePages],
) -> Result<(), UserMessage> {
    for language in languages.iter_mut() {
        let LanguagePages {
            site,
            pages,
            unlocked,
            ..
        } = language;
        let mut links: Option<Wikilinks> = None;
        for page in pages.iter() {
            let doc = content
                .posts
                .iter()
                .find(|doc| page.source.as_ref() == Some(&doc.path));
            let Some(doc) = doc.filter(|doc| Lock::of(&doc.front).is_none()) else {
                continue;
            };
            let PageBody::Post(post) = &page.body else {
                continue;
            };
            if !post.body.as_str().contains(LOCKED) {
                continue;
            }
            let links = links.get_or_insert_with(|| {
                Wikilinks::for_language(content, config, routes, &site.locale)
            });
            let rendered = render_gated(&doc.body, &doc.locale, config.slugs(), links, Gated::Open)
                .map_err(|error| build_message::block(&doc.path, &error))?;
            let parts = build_seo::with_link_policy(config, rendered.html);
            let Some(html) = opened(post.body.as_str(), &parts) else {
                continue;
            };
            let mut page = page.clone();
            if let PageBody::Post(post) = &mut page.body {
                post.body = BodyHtml::from_sanitised(html);
                post.reading_time = ReadingTime::of(&post.body, &site.locale);
            }
            let key = Key::Reader;
            unlocked.push(Unlocked { page, key });
        }
    }
    Ok(())
}

/// `public`, a finished body, with each locked gated part swapped in turn
/// for the open part of `open`, the body rendered with its gated parts
/// open. `None` when the two do not hold as many parts, so a page never
/// shows one part in place of another.
fn opened(public: &str, open: &str) -> Option<String> {
    let mut parts = open.split(OPEN).skip(1).map(|rest| rest.split_once(END));
    let mut html = String::with_capacity(public.len() + open.len());
    let mut rest = public;
    while let Some((before, after)) = rest.split_once(LOCKED) {
        let (_, after) = after.split_once(END)?;
        let (part, _) = parts.next()??;
        html.push_str(before);
        html.push_str(OPEN);
        html.push_str(part);
        html.push_str(END);
        rest = after;
    }
    html.push_str(rest);
    parts.next().is_none().then_some(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_locked_part_takes_its_open_part_in_turn() {
        let public = "<p>Intro</p>\n<section class=\"gated\">\n<p>Note</p>\n<form></form>\n</section>\n<p>Middle</p>\n<section class=\"gated\">\n<p>Two</p>\n</section>\n<p>End</p>";
        let open = "<p>x</p>\n<section class=\"gated gated-open\">\n<p>First secret</p>\n</section>\n<p>y</p>\n<section class=\"gated gated-open\">\n<p>Second secret</p>\n</section>\n";
        assert_eq!(
            opened(public, open).unwrap(),
            "<p>Intro</p>\n<section class=\"gated gated-open\">\n<p>First secret</p>\n</section>\n<p>Middle</p>\n<section class=\"gated gated-open\">\n<p>Second secret</p>\n</section>\n<p>End</p>"
        );
    }

    #[test]
    fn parts_that_do_not_match_open_nothing() {
        let public = "<section class=\"gated\">\n<p>Note</p>\n</section>\n";
        let none = "<p>No parts</p>\n";
        let two = "<section class=\"gated gated-open\">\n<p>A</p>\n</section>\n<section class=\"gated gated-open\">\n<p>B</p>\n</section>\n";
        assert_eq!(opened(public, none), None);
        assert_eq!(opened(public, two), None);
    }
}
