//! The small prebuilt search index `khalid build` writes for a static site,
//! one file per language beside its search page, `search/index.json`: each
//! published post's address, title, description and terms. Plain files
//! cannot answer a query, so the page says search is not available; the file is what
//! a search in the browser can read on a small site (plan section 7).

use std::collections::BTreeSet;

use khalid_core::{Config, Mode, UserMessage};
use khalid_render::PageBody;
use khalid_search::{Analyzer, Language};
use serde::Serialize;

use crate::build_files::SiteFile;
use crate::build_pages::{self, LanguagePages};
use crate::{search_page, serve_message};

/// One language's index file.
#[derive(Serialize)]
struct Index<'a> {
    language: &'a str,
    pages: Vec<Entry<'a>>,
}

/// One published post in the index file.
#[derive(Serialize)]
struct Entry<'a> {
    href: &'a str,
    title: &'a str,
    description: &'a str,
    /// The terms of the title and the description, each once, in order.
    terms: Vec<String>,
}

/// The index file of each of `languages`, or none unless the site is
/// static: a server answers searches itself, and a headless site has no
/// pages.
///
/// # Errors
///
/// Returns what, why and fix if an index cannot be written as JSON, which
/// does not happen with text.
pub(crate) fn files(
    config: &Config,
    languages: &[LanguagePages],
) -> Result<Vec<SiteFile>, UserMessage> {
    if config.mode() != Mode::Static {
        return Ok(Vec::new());
    }
    languages
        .iter()
        .map(|language| {
            let locale = &language.site.locale;
            let analyzer = Analyzer::new(Language::of(locale));
            let posts = language
                .pages
                .iter()
                .filter(|page| matches!(page.body, PageBody::Post(_)));
            let pages = posts
                .map(|page| {
                    let words = format!("{}\n{}", page.meta.title, page.meta.description);
                    let terms: BTreeSet<String> = analyzer
                        .terms(&words)
                        .map(|term| term.as_str().to_owned())
                        .collect();
                    Entry {
                        href: &page.meta.path,
                        title: &page.meta.title,
                        description: &page.meta.description,
                        terms: terms.into_iter().collect(),
                    }
                })
                .collect();
            let index = Index {
                language: locale.as_str(),
                pages,
            };
            let text = serde_json::to_string(&index).map_err(|_| serve_message::internal())?;
            let prefix = build_pages::prefix(config, locale);
            Ok(SiteFile {
                file: format!("{prefix}{}index.json", search_page::REST),
                text,
            })
        })
        .collect()
}
