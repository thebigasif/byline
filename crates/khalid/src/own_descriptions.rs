//! The meta description of each page Khalid makes itself: the home page,
//! the lists, the author pages, the glossary index, search and the sitemap
//! page. A writer has no file to describe them in, so without these the
//! audit warns about a brand-new site's own pages (KH-LNT-5).

use khalid_core::{Config, UserMessage};
use khalid_render::{PageBody, Renderer};

use crate::build_lists;
use crate::build_pages::{LanguagePages, Page};

/// Gives each page of `languages` that Khalid makes and that has no
/// description one in its language, from `renderer`'s strings, naming the
/// site, and the page's title where a language has many such pages, so no
/// two pages share it. A description the
/// page already has, or a `[seo.titles]` template that fills one, wins:
/// this is the page's excerpt, which a template only falls back to.
///
/// # Errors
///
/// A [`UserMessage`] when a language's strings lack a description, which
/// the strings test rules out.
pub(crate) fn fill(
    config: &Config,
    renderer: &Renderer,
    languages: &mut [LanguagePages],
) -> Result<(), UserMessage> {
    for language in languages {
        let locale = &language.site.locale;
        for page in &mut language.pages {
            if !page.seo.excerpt.is_empty() || !page.meta.description.is_empty() {
                continue;
            }
            let Some(key) = key(page) else {
                continue;
            };
            let text = build_lists::text(renderer, locale, key)?
                .replace("{site}", config.name())
                .replace("{title}", &page.meta.title);
            page.seo.excerpt.clone_from(&text);
            page.meta.description = text;
        }
    }
    Ok(())
}

/// The strings key of `page`'s description, or `None` for a page a writer
/// describes in its own file, such as a post.
fn key(page: &Page) -> Option<&'static str> {
    match &page.body {
        PageBody::Home(_) => Some("home_description"),
        PageBody::Collection(_)
        | PageBody::Tag(_)
        | PageBody::Category(_)
        | PageBody::Archive(_)
        | PageBody::Channel(_)
        | PageBody::Date(_) => Some("list_description"),
        PageBody::Sitemap(_) => Some("sitemap_description"),
        PageBody::Authors(_) => Some("authors_description"),
        PageBody::Author(_) => Some("author_description"),
        PageBody::Glossary(_) => Some("glossary_description"),
        PageBody::Search(_) => Some("search_description"),
        // A glossary term's own definition should describe it, and a
        // sentence about the site would land in its schema as one.
        _ => None,
    }
}
