//! `khalid lint --fix`: the safe repairs, made in the post files.
//!
//! A repair runs only for a finding the linter reported, so a rule turned off
//! in `lint.toml` is never repaired. The file is replaced in one step, so a
//! crash never leaves half a post.

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Component, Path};

use khalid_content::{Document, PostFront};
use khalid_core::{Config, SitePath, UserMessage};
use khalid_lint::{Finding, LintSite};
use khalid_routes::Routes;
use serde::Serialize;

use crate::repair::{Change, INSECURE_INTERNAL_LINK, RULES, repaired_body};
use crate::slug_repair::{Move, SLUG_RULES, moved_header};

/// One repair made in one post file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Repair {
    /// The post file, from the site folder, with `/` between folders.
    pub(crate) path: String,
    /// The rule whose finding it repairs.
    pub(crate) rule: &'static str,
    /// The heading or image it changed.
    pub(crate) subject: String,
    /// What changed, as one plain sentence.
    pub(crate) change: String,
}

/// Makes the safe repairs for `findings` in the files of `documents`, which
/// sit in the same order as the posts of `lint_site`, and returns what it
/// changed. `config` gives the slug mode and, with the site's routes, the
/// address a cleaned slug moves away from.
///
/// # Errors
///
/// Returns a message when a changed file cannot be written.
pub(crate) fn repair(
    site: &Path,
    lint_site: &LintSite,
    documents: &[&Document<PostFront>],
    findings: &[Finding],
    config: &Config,
) -> Result<Vec<Repair>, UserMessage> {
    let routes = Routes::load(site).ok();
    let mut repairs = Vec::new();
    for (post, document) in lint_site.posts.iter().zip(documents) {
        let own: Vec<&Finding> = findings
            .iter()
            .filter(|finding| {
                let at = finding.location();
                at.locale == post.locale && at.slug == post.slug
            })
            .collect();
        let reported = |rule: &&'static str| own.iter().any(|f| f.rule().as_str() == *rule);
        let insecure: Vec<&str> = own
            .iter()
            .filter(|finding| finding.rule().as_str() == INSECURE_INTERNAL_LINK)
            .filter_map(|finding| finding.location().subject.as_deref())
            .collect();
        let rules: Vec<&'static str> = RULES.into_iter().filter(reported).collect();
        let slug_rules: Vec<&'static str> = SLUG_RULES.into_iter().filter(reported).collect();
        let (Some(path), false) = (
            slashed(&document.path),
            rules.is_empty() && slug_rules.is_empty(),
        ) else {
            continue;
        };
        let edit = |header: &str| {
            let body = &document.body;
            let mut changes = Vec::new();
            let repaired = repaired_body(body, &rules, &insecure, &post.locale, config.slugs());
            let body = repaired.map_or_else(
                || body.clone(),
                |(fixed, made)| {
                    changes.extend(made);
                    fixed
                },
            );
            let moved = routes
                .as_ref()
                .zip(collection(&document.path))
                .filter(|_| !slug_rules.is_empty())
                .and_then(|(routes, collection)| {
                    let moving = Move {
                        rules: &slug_rules,
                        post,
                        collection,
                        site: lint_site,
                    };
                    moved_header(header, &body, &moving, config, routes)
                });
            let header = moved.map_or_else(
                || header.to_owned(),
                |(fixed, made)| {
                    changes.extend(made);
                    fixed
                },
            );
            (!changes.is_empty()).then(|| (format!("{header}{body}"), changes))
        };
        let changes = rewrite(site, &path, &document.body, edit)?;
        repairs.extend(changes.into_iter().map(|change| Repair {
            path: path.clone(),
            rule: change.rule,
            subject: change.subject,
            change: change.sentence,
        }));
    }
    Ok(repairs)
}

/// Rewrites the post file `path` with what `edit` makes of its header, the
/// text before `body`, through a file beside it that is synced and renamed
/// over it (BL-RUN-9). Writes nothing when `edit` changes nothing or the
/// file no longer ends with `body`.
fn rewrite(
    site: &Path,
    path: &str,
    body: &str,
    edit: impl FnOnce(&str) -> Option<(String, Vec<Change>)>,
) -> Result<Vec<Change>, UserMessage> {
    let failed = |_: io::Error| unwritable(path);
    let file = SitePath::within(site, path).map_err(|_| unwritable(path))?;
    let spare = SitePath::within(site, &format!("{path}.khalid-fix"));
    let spare = spare.map_err(|_| unwritable(path))?;
    let file = file.resolve().map_err(|_| unwritable(path))?;
    let spare = spare.resolve().map_err(|_| unwritable(path))?;
    let text = fs::read_to_string(&file).map_err(failed)?;
    let Some((new, changes)) = text.strip_suffix(body).and_then(edit) else {
        return Ok(Vec::new());
    };
    let written = File::create(&spare).and_then(|mut out| {
        out.write_all(new.as_bytes())?;
        out.sync_all()
    });
    if let Err(error) = written.and_then(|()| fs::rename(&spare, &file)) {
        let _ = fs::remove_file(&spare);
        return Err(failed(error));
    }
    Ok(changes)
}

/// The collection a post file sits in: `posts` for
/// `content/en/posts/my-post.md`.
fn collection(path: &Path) -> Option<&str> {
    path.components().nth(2)?.as_os_str().to_str()
}

/// `path` with `/` between its folders, or `None` when a part is not text.
fn slashed(path: &Path) -> Option<String> {
    let parts: Option<Vec<&str>> = path
        .components()
        .map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    Some(parts?.join("/"))
}

/// The message for a post file `--fix` could not write.
fn unwritable(path: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not save its repair to `{path}`."),
        "The file is unchanged, so the problem it found is still there.",
        "Check that you can write to the file, then run `khalid lint --fix` again.",
    )
}
