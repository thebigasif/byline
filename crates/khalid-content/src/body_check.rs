//! The checks on every body that need the whole site, such as wikilinks
//! reaching a page.

use comrak::nodes::NodeValue;
use comrak::{Arena, parse_document};
use khalid_core::{Config, Locale};
use khalid_routes::Routes;

use crate::render::options;
use crate::{ContentError, Document, SiteContent, Wikilinks};

/// Checks every content file of `site`'s languages: each wikilink reaches a
/// page, as [`Wikilinks::for_language`] finds them with `routes`.
///
/// # Errors
///
/// Returns [`ContentError::Wikilink`] for the first wikilink that reaches
/// nothing, in language order and then in the order of [`SiteContent`]'s
/// fields, with the file and its line.
pub fn check_bodies(
    content: &SiteContent,
    site: &Config,
    routes: &Routes,
) -> Result<(), ContentError> {
    for locale in site.languages() {
        let links = Wikilinks::for_language(content, site, routes, locale);
        let check = Check {
            links: &links,
            locale,
        };
        check.all(&content.posts)?;
        check.all(&content.pages)?;
        check.all(&content.authors)?;
        check.all(&content.glossary)?;
        check.all(&content.case_studies)?;
        check.all(&content.changelog)?;
        check.all(&content.snippets)?;
        check.all(&content.indexes)?;
    }
    Ok(())
}

/// The wikilinks of one language and the targets they may reach.
struct Check<'a> {
    links: &'a Wikilinks,
    locale: &'a Locale,
}

impl Check<'_> {
    /// Checks every document of `docs` written in this language.
    fn all<F>(&self, docs: &[Document<F>]) -> Result<(), ContentError> {
        for doc in docs.iter().filter(|doc| &doc.locale == self.locale) {
            let arena = Arena::new();
            let root = parse_document(&arena, &doc.body, &options());
            for node in root.descendants() {
                let data = node.data.borrow();
                let line = doc
                    .body_line
                    .saturating_add(data.sourcepos.start.line)
                    .saturating_sub(1);
                match &data.value {
                    NodeValue::WikiLink(wikilink) if self.links.get(&wikilink.url).is_none() => {
                        return Err(ContentError::Wikilink {
                            path: doc.path.clone(),
                            line,
                            target: wikilink.url.clone(),
                            language: self.locale.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
