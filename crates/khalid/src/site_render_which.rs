//! Which pages a render of the running site makes, and how their HTML is
//! rendered into the pages `khalid serve` answers with.

use khalid_core::{Config, UserMessage};
use khalid_render::{PageBody, Renderer};
use khalid_server::Page;

use crate::build_pages::LanguagePages;
use crate::post_lock;
use crate::site_page::HeadNonce;

/// Which pages a render makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    /// Every page, the stylesheet and the not-found page.
    All,
    /// Only the page of each dated post.
    Posts,
}

/// Renders the `which` pages of `languages` with `renderer` into `pages`,
/// and returns the not-found page of the default language, empty unless
/// `which` is [`Which::All`].
///
/// # Errors
///
/// A [`UserMessage`] when a page does not render or has no address.
pub(crate) fn html_pages(
    renderer: &Renderer,
    config: &Config,
    languages: &[LanguagePages],
    which: Which,
    pages: &mut Vec<Page>,
) -> Result<String, UserMessage> {
    let mut not_found = String::new();
    for language in languages {
        for page in &language.pages {
            if which == Which::Posts && !matches!(page.body, PageBody::Post(_)) {
                continue;
            }
            let html = page.rendered(renderer, &language.site, HeadNonce::PerRequest)?;
            if matches!(page.body, PageBody::NotFound) {
                if &language.site.locale == config.default_language() {
                    not_found = html;
                }
                continue;
            }
            pages.push(page.served_at(&page.meta.path, html)?);
        }
        for unlocked in &language.unlocked {
            let page = &unlocked.page;
            let html = page.rendered(renderer, &language.site, HeadNonce::PerRequest)?;
            pages.push(page.served_at(&post_lock::hidden(&page.meta.path), html)?);
        }
    }
    Ok(not_found)
}
