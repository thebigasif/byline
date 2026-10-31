//! The checks on every body that need the whole site: wikilinks reach a
//! page, snippets are the site's, and formulas can be drawn.

use comrak::nodes::NodeValue;
use comrak::{Arena, parse_document};
use khalid_core::{Config, Locale};
use khalid_routes::Routes;

use crate::render::options;
use crate::{BlockKind, ContentError, Document, SiteContent, Wikilinks, maths, parse_blocks};

/// Checks every content file of `site`'s languages: each wikilink reaches a
/// page, as [`Wikilinks::for_language`] finds them with `routes`, and each
/// formula can be drawn as `MathML`.
///
/// # Errors
///
/// Returns [`ContentError::Wikilink`] for a wikilink that reaches nothing and
/// [`ContentError::Maths`] for a formula that cannot be drawn, the first in
/// language order and then in the order of [`SiteContent`]'s fields, with
/// the file and its line.
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
                    NodeValue::Math(math) => {
                        maths::mathml(math).map_err(|problem| ContentError::Maths {
                            path: doc.path.clone(),
                            line,
                            problem,
                        })?;
                    }
                    _ => {}
                }
            }
            self.snippets(doc)?;
        }
        Ok(())
    }

    /// Checks that each snippet `doc` places is one of the site's, in its
    /// own language. A body whose blocks cannot be read is left to the
    /// reader that renders it, which names the block's problem.
    fn snippets<F>(&self, doc: &Document<F>) -> Result<(), ContentError> {
        let Ok(blocks) = parse_blocks(&doc.body) else {
            return Ok(());
        };
        for block in blocks {
            let BlockKind::Snippet { slug } = block.kind else {
                continue;
            };
            if self.links.snippet(&slug).is_some() {
                continue;
            }
            let at = doc.body.find(&block.source).unwrap_or_default();
            let before = doc.body.get(..at).unwrap_or_default().matches('\n');
            return Err(ContentError::Snippet {
                path: doc.path.clone(),
                line: doc.body_line.saturating_add(before.count()),
                slug,
                language: self.locale.clone(),
            });
        }
        Ok(())
    }
}
