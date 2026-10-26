//! The address repair: a slug without the filler words and years the slug
//! rules flag. The old address goes into `aliases`, so it redirects to the
//! new one and links and rankings follow.

use std::path::Path;

use khalid_content::parse_post;
use khalid_core::{Config, Slug};
use khalid_lint::{LintPost, LintSite, RuleId, flags_slug_part};
use khalid_routes::{PostAddress, Routes};

use crate::repair::Change;

/// The rules the address repair answers, in the order it reports them.
pub(crate) const SLUG_RULES: [&str; 2] = ["slug-stopwords", "slug-date"];

/// One post whose address may move.
pub(crate) struct Move<'a> {
    /// The slug rules the linter reported for it.
    pub(crate) rules: &'a [&'static str],
    /// The post, with its language, slug and date.
    pub(crate) post: &'a LintPost,
    /// Its collection, such as `posts`.
    pub(crate) collection: &'a str,
    /// The site, whose posts and pages hold the slugs already taken.
    pub(crate) site: &'a LintSite,
}

/// The post's `header` with the cleaned slug and the old address added to
/// `aliases`, and the changes, or `None` when the slug stays, the new one is
/// taken, or the header cannot be changed safely. `body` follows the header
/// in the file, so the result is checked by reading the whole post again.
pub(crate) fn moved_header(
    header: &str,
    body: &str,
    moving: &Move<'_>,
    config: &Config,
    routes: &Routes,
) -> Option<(String, Vec<Change>)> {
    let post = moving.post;
    let language = post.locale.language();
    let flagged =
        |rule: &'static str, part: &str| flags_slug_part(RuleId::new(rule), language, part);
    let parts: Vec<&str> = post.slug.as_str().split('-').collect();
    let removed = |rule| -> Vec<&str> {
        parts
            .iter()
            .copied()
            .filter(|part| flagged(rule, part))
            .collect()
    };
    let keep = |part: &&str| !moving.rules.iter().any(|rule| flagged(rule, part));
    let kept: Vec<&str> = parts.iter().copied().filter(keep).collect();
    if kept.is_empty() || kept.len() == parts.len() {
        return None;
    }
    let slug = Slug::from_title(&kept.join("-"), &post.locale, config.slugs()).ok()?;
    let site = moving.site;
    let posts = site.posts.iter().map(|p| (&p.locale, &p.slug));
    let mut taken = posts.chain(site.pages.iter().map(|p| (&p.locale, &p.slug)));
    if taken.any(|other| other == (&post.locale, &slug)) {
        return None;
    }
    let address = PostAddress::new(moving.collection, &post.locale, &post.slug, post.date);
    let old = routes.address(config, &address).ok()?;
    let header = edited(header, &slug, &old)?;
    let front = parse_post(Path::new("post.md"), &format!("{header}{body}"))
        .ok()?
        .front;
    if front.slug.as_ref() != Some(&slug) || !front.aliases.contains(&old) {
        return None;
    }
    let changes = moving.rules.iter().map(|&rule| {
        let parts = removed(rule).join(", ");
        let what = if rule == "slug-date" {
            format!("The address no longer has the year {parts}.")
        } else {
            format!("The address no longer has the filler words {parts}.")
        };
        Change {
            rule,
            subject: post.slug.as_str().to_owned(),
            sentence: format!("{what} It ends in {slug}, and {old} now redirects to it."),
        }
    });
    Some((header, changes.collect()))
}

/// `header` with `slug:` set to `slug` and `old` first in `aliases`, or
/// `None` for a header in a form this repair does not edit: lines that end
/// in `\r`, or `aliases` written on one line.
fn edited(header: &str, slug: &Slug, old: &str) -> Option<String> {
    if header.contains('\r') {
        return None;
    }
    let mut lines: Vec<String> = header.split_inclusive('\n').map(str::to_owned).collect();
    let mut extra = Vec::new();
    match lines.iter().position(|line| line.starts_with("slug:")) {
        Some(at) => *lines.get_mut(at)? = format!("slug: {slug}\n"),
        None => extra.push(format!("slug: {slug}\n")),
    }
    match lines.iter().position(|line| line.starts_with("aliases:")) {
        Some(at) => {
            if !lines
                .get(at)?
                .trim_start_matches("aliases:")
                .trim()
                .is_empty()
            {
                return None;
            }
            let next = lines.get(at.checked_add(1)?)?;
            let indent = match next.strip_suffix(next.trim_start()) {
                Some(indent) if next.trim_start().starts_with("- ") => indent,
                _ => "  ",
            };
            let item = format!("{indent}- {old}\n");
            lines.insert(at.checked_add(1)?, item);
        }
        None => extra.push(format!("aliases:\n  - {old}\n")),
    }
    let close = lines
        .iter()
        .skip(1)
        .position(|line| line.trim_end() == "---")?
        .checked_add(1)?;
    let tail = lines.split_off(close);
    Some([lines, extra, tail].concat().concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(text: &str) -> Slug {
        Slug::try_new(text).unwrap()
    }

    #[test]
    fn a_missing_slug_and_aliases_go_before_the_closing_fence() {
        let header = "---\ntitle: A\n---\n\n";
        let edited = edited(header, &slug("a"), "/blog/the-a/").unwrap();
        assert_eq!(
            edited,
            "---\ntitle: A\nslug: a\naliases:\n  - /blog/the-a/\n---\n\n"
        );
    }

    #[test]
    fn aliases_on_one_line_are_left_for_the_writer() {
        let header = "---\naliases: [/blog/x/]\n---\n";
        assert!(edited(header, &slug("a"), "/blog/the-a/").is_none());
    }
}
