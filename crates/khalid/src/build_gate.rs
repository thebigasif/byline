//! The publish gate in `khalid build` and `khalid serve`: a post the gate
//! blocks is left out of the site, and the rest of the site goes live, so
//! one old post never empties a site and the two commands always agree on
//! what is published (KH-LNT-1).
//!
//! A post with no date is a draft. Neither command shows it, so the gate
//! leaves it alone too.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use khalid_content::SiteContent;
use khalid_core::{Clock, Config, UserMessage};
use khalid_lint::{LintFact, Linter, PublishGate, Report, Verdict};
use khalid_routes::Routes;

use crate::lint_input::{self, LintInput};
use crate::site_briefs::Briefs;
use crate::{lint, lint_message, site_briefs, site_facts};

/// A dated post the gate keeps off the site, and the report saying why.
#[derive(Debug)]
pub(crate) struct Refused {
    /// The post's file, relative to the site folder.
    pub(crate) path: PathBuf,
    /// Every finding about the post, warnings included.
    pub(crate) report: Report,
}

/// Every dated post of `content` the gate refuses, judged with the `site`
/// folder's `lint.toml`, fact registry and briefs, on today's date from
/// `clock`.
///
/// # Errors
///
/// What, why and fix when `lint.toml`, the fact registry or a brief cannot
/// be read.
pub(crate) fn screen(
    site: &Path,
    (config, routes, content): (&Config, &Routes, &SiteContent),
    clock: &dyn Clock,
) -> Result<Vec<Refused>, UserMessage> {
    let settings = lint::lint_settings(site)?;
    let linter = Linter::new(&settings, clock).map_err(|error| lint_message::of(&error))?;
    let facts = site_facts::load(site)?;
    let briefs = site_briefs::load(site)?;
    Ok(refused(&linter, content, config, routes, (facts, briefs)))
}

/// Each dated post in `content` that `gate` refuses, in the order the
/// posts were read. Empty when every post may go live. Each post first
/// gets the quick yes or no, and only a refused post the full verdict with
/// its warnings, so a 10,000-post site stays inside its budget. Posts are
/// held to `facts`, the site's fact registry, and to their `briefs`.
pub(crate) fn refused(
    gate: &dyn PublishGate,
    content: &SiteContent,
    config: &Config,
    routes: &Routes,
    (facts, briefs): (Vec<LintFact>, Briefs),
) -> Vec<Refused> {
    let input = LintInput::new(content, config, routes.clone())
        .with_facts(facts)
        .with_briefs(briefs);
    let (site, documents) = lint_input::lint_site(content, config, &input);
    site.posts
        .iter()
        .zip(documents)
        .filter(|(post, _)| post.date.is_some() && gate.refuses(post, &site))
        .filter_map(|(post, document)| match gate.verdict(post, &site) {
            Verdict::Refuse { report } => Some(Refused {
                path: document.path.clone(),
                report,
            }),
            Verdict::Publish { .. } => None,
        })
        .collect()
}

/// `content` without the `refused` posts. Borrowed as it is when the gate
/// refused none.
pub(crate) fn published<'c>(content: &'c SiteContent, refused: &[Refused]) -> Cow<'c, SiteContent> {
    if refused.is_empty() {
        return Cow::Borrowed(content);
    }
    let mut kept = content.clone();
    kept.posts
        .retain(|doc| !refused.iter().any(|post| post.path == doc.path));
    Cow::Owned(kept)
}

/// `content` as `khalid serve` shows it: without the posts the gate
/// refuses, as [`screen`] judges them, so readers see what `khalid build`
/// writes. Logs each post left out.
///
/// # Errors
///
/// What, why and fix when the site's lint settings cannot be read.
pub(crate) fn for_readers<'c>(
    site: &Path,
    (config, routes, content): (&Config, &Routes, &'c SiteContent),
    clock: &dyn Clock,
) -> Result<Cow<'c, SiteContent>, UserMessage> {
    let refused = screen(site, (config, routes, content), clock)?;
    for post in &refused {
        let post = post.path.display();
        tracing::warn!(%post, "a post is not ready to publish, so readers do not see it; `khalid lint` says why");
    }
    Ok(published(content, &refused))
}

/// Every finding of each post in `refused`, then what was left out, why
/// and the fix, for a person to read after a build. Empty when nothing was
/// left out.
pub(crate) fn human(refused: &[Report]) -> String {
    let mut text = String::new();
    if refused.is_empty() {
        return text;
    }
    for finding in refused.iter().flat_map(Report::findings) {
        lint::write_finding(&mut text, finding);
    }
    let count = match refused.len() {
        1 => "1 post, because it is".to_owned(),
        posts => format!("{posts} posts, because they are"),
    };
    format!(
        "{text}Left out {count} not ready to publish. The blocks are listed above.\nFix each block and build again, or move a post to drafts/<language>/ to keep it a draft.\n"
    )
}
