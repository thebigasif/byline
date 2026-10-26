//! The `_redirects` file `khalid build` writes from the `aliases` in post
//! front matter, so a moved post keeps its links on a static host (KH-SEO-8,
//! ADR "the build writes its redirects as a `_redirects` file").

use khalid_content::SiteContent;
use khalid_core::{Config, UserMessage};
use khalid_seo::{Destination, Move, Redirect, RedirectRule, RedirectTable, UrlPath};

use crate::build_files::SiteFile;
use crate::build_message;
use crate::build_pages::{named, prefix};

/// The redirects file, or `None` when no post has an alias.
pub(crate) fn file(
    config: &Config,
    content: &SiteContent,
) -> Result<Option<SiteFile>, UserMessage> {
    let mut rules = Vec::new();
    for doc in content.posts.iter().filter(|doc| doc.front.date.is_some()) {
        let front = &doc.front;
        let (_, slug) = named(config, doc, front.slug.as_ref(), front.title.as_deref())?;
        let to = path(&format!("/{}{slug}/", prefix(config, &doc.locale)))?;
        for alias in &front.aliases {
            let inside = alias
                .strip_prefix(config.mount().trim_end_matches('/'))
                .filter(|rest| rest.starts_with('/'))
                .ok_or_else(|| build_message::alias_outside(&doc.path, alias, config.mount()))?;
            let redirect = Redirect::Move {
                kind: Move::Permanent,
                to: Destination::Path(to.clone()),
            };
            rules.push(RedirectRule::new(path(inside)?, redirect));
        }
    }
    if rules.is_empty() {
        return Ok(None);
    }
    let table = RedirectTable::new(rules).map_err(|error| build_message::seo(&error))?;
    Ok(Some(SiteFile {
        file: "_redirects".to_owned(),
        text: lines(config, &table),
    }))
}

fn path(text: &str) -> Result<UrlPath, UserMessage> {
    UrlPath::try_new(text).map_err(|error| build_message::seo(&error))
}

/// One `from to status` line per rule, with each chain followed to its end
/// and every path from the site root, mount folder included.
fn lines(config: &Config, table: &RedirectTable) -> String {
    let mount = config.mount().trim_end_matches('/');
    table
        .rules()
        .iter()
        .filter_map(|rule| {
            let answer = table.resolve(&rule.from)?;
            let to = match answer.location()? {
                Destination::Path(to) => format!("{mount}{to}"),
                Destination::External(url) => url.to_string(),
                _ => return None,
            };
            Some(format!("{mount}{} {to} {}\n", rule.from, answer.status()))
        })
        .collect()
}
