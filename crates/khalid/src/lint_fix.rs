//! `khalid lint --fix`: the safe repairs, made in the post files.
//!
//! A repair runs only for a finding the linter reported, so a rule turned off
//! in `lint.toml` is never repaired. The file is replaced in one step, so a
//! crash never leaves half a post.

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Component, Path};

use khalid_content::{Document, PostFront};
use khalid_core::{SitePath, SlugMode, UserMessage};
use khalid_lint::{Finding, LintPost};
use serde::Serialize;

use crate::repair::{RULES, repaired_body};

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
/// sit in the same order as `posts`, and returns what it changed. `slugs` is
/// the site's slug mode, which captions render with.
///
/// # Errors
///
/// Returns a message when a changed file cannot be written.
pub(crate) fn repair(
    site: &Path,
    posts: &[LintPost],
    documents: &[&Document<PostFront>],
    findings: &[Finding],
    slugs: SlugMode,
) -> Result<Vec<Repair>, UserMessage> {
    let mut repairs = Vec::new();
    for (post, document) in posts.iter().zip(documents) {
        let reported = |rule: &&str| {
            findings.iter().any(|finding| {
                let at = finding.location();
                finding.rule().as_str() == *rule && at.locale == post.locale && at.slug == post.slug
            })
        };
        let rules: Vec<&str> = RULES.into_iter().filter(reported).collect();
        if rules.is_empty() {
            continue;
        }
        let Some(path) = slashed(&document.path) else {
            continue;
        };
        let Some((body, changes)) = repaired_body(&document.body, &rules, &post.locale, slugs)
        else {
            continue;
        };
        if write_body(site, &path, &document.body, &body)? {
            repairs.extend(changes.into_iter().map(|change| Repair {
                path: path.clone(),
                rule: change.rule,
                subject: change.subject,
                change: change.sentence,
            }));
        }
    }
    Ok(repairs)
}

/// Replaces `old` with `new` at the end of the file `path`, through a file
/// beside it that is synced and renamed over it (BL-RUN-9). Returns `false`,
/// and writes nothing, when the file no longer ends with `old`.
fn write_body(site: &Path, path: &str, old: &str, new: &str) -> Result<bool, UserMessage> {
    let failed = |_: io::Error| unwritable(path);
    let file = SitePath::within(site, path).map_err(|_| unwritable(path))?;
    let spare = SitePath::within(site, &format!("{path}.khalid-fix"));
    let spare = spare.map_err(|_| unwritable(path))?;
    let file = file.resolve().map_err(|_| unwritable(path))?;
    let spare = spare.resolve().map_err(|_| unwritable(path))?;
    let text = fs::read_to_string(&file).map_err(failed)?;
    let Some(header) = text.strip_suffix(old) else {
        return Ok(false);
    };
    let written = File::create(&spare).and_then(|mut out| {
        out.write_all(format!("{header}{new}").as_bytes())?;
        out.sync_all()
    });
    if let Err(error) = written.and_then(|()| fs::rename(&spare, &file)) {
        let _ = fs::remove_file(&spare);
        return Err(failed(error));
    }
    Ok(true)
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
