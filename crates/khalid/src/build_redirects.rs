//! The `_redirects` file `khalid build` writes from the `aliases` in post
//! front matter and the moves stored in `data/site.db`, so a moved post
//! keeps its links on a static host (KH-SEO-8, ADR "the build writes its
//! redirects as a `_redirects` file").

use std::path::Path;

use khalid_content::SiteContent;
use khalid_core::{Config, UserMessage};
use khalid_routes::{PostAddress, Routes};
use khalid_seo::{Destination, Move, Redirect, RedirectRule, RedirectTable, UrlPath};

use crate::build_files::SiteFile;
use crate::build_pages::{named, prefix, routed};
use crate::{build_message, db, redirects};

/// One alias and the address it ends at, each from the site root, mount
/// folder included. Every alias is a permanent move.
#[derive(Debug)]
pub(crate) struct Hop {
    pub(crate) from: String,
    pub(crate) to: String,
}

/// The redirects file, or `None` when there is nothing to redirect: one
/// line per alias, then one per move stored in the `site` folder's
/// database, if it has one, each chain followed to its end. Stored 410,
/// 451 and regex rules have no line a static host reads, so they stay
/// with `khalid serve`.
pub(crate) fn file(
    site: &Path,
    config: &Config,
    routes: &Routes,
    content: &SiteContent,
) -> Result<Option<SiteFile>, UserMessage> {
    let mut rules = Vec::new();
    for hop in hops(config, routes, content)? {
        let to = Destination::Path(path(&hop.to)?);
        let redirect = Redirect::Move {
            kind: Move::Permanent,
            to,
        };
        rules.push(RedirectRule::new(path(&hop.from)?, redirect));
    }
    rules.extend(stored_moves(site)?);
    if rules.is_empty() {
        return Ok(None);
    }
    let table = RedirectTable::new(rules).map_err(|error| build_message::seo(&error))?;
    let lines = table.rules().iter().filter_map(|rule| {
        let hit = table.resolve(&rule.from)?;
        let to = hit.location()?;
        Some(format!("{} {to} {}\n", rule.from, hit.status()))
    });
    Ok(Some(SiteFile {
        file: "_redirects".to_owned(),
        text: lines.collect(),
    }))
}

/// The exact moves stored in `data/site.db` in `site`, or none when the
/// site has no database; a build never makes one.
fn stored_moves(site: &Path) -> Result<Vec<RedirectRule>, UserMessage> {
    if !db::data_file(site)?.exists() {
        return Ok(Vec::new());
    }
    let read = redirects::on_desk(site, async |desk| desk.table().await);
    let table = read.map_err(|(message, _)| message)?.map_err(|refusal| {
        UserMessage::new(
            format!("The stored redirects cannot be written to _redirects: {refusal}."),
            "A static host would send readers round in a loop or nowhere.",
            "Run khalid redirects export to see them, fix the list, and import it again.",
        )
    })?;
    let moves = table
        .rules()
        .iter()
        .filter(|rule| rule.redirect.location().is_some());
    Ok(moves.cloned().collect())
}

/// Every alias of a dated post, moved to the address `routes` gives the
/// post, as its page has, with each chain followed to its end, in the
/// order of the redirect table. `khalid serve` answers the same hops.
pub(crate) fn hops(
    config: &Config,
    routes: &Routes,
    content: &SiteContent,
) -> Result<Vec<Hop>, UserMessage> {
    let mut rules = Vec::new();
    for doc in content.posts.iter().filter(|doc| doc.front.date.is_some()) {
        let front = &doc.front;
        let (_, slug) = named(config, doc, front.slug.as_ref(), front.title.as_deref())?;
        let entry = PostAddress::new(&doc.collection, &doc.locale, &slug, front.date);
        let rest = routed(config, routes, &doc.locale, &entry, &doc.path)?;
        let to = path(&format!("/{}{rest}", prefix(config, &doc.locale)))?;
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
        return Ok(Vec::new());
    }
    let table = RedirectTable::new(rules).map_err(|error| build_message::seo(&error))?;
    Ok(followed(config, &table))
}

fn path(text: &str) -> Result<UrlPath, UserMessage> {
    UrlPath::try_new(text).map_err(|error| build_message::seo(&error))
}

/// One hop per rule, with each chain followed to its end and every path
/// from the site root, mount folder included.
fn followed(config: &Config, table: &RedirectTable) -> Vec<Hop> {
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
            Some(Hop {
                from: format!("{mount}{}", rule.from),
                to,
            })
        })
        .collect()
}
