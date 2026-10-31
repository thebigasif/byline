//! Who `khalid new` names as a post's author: the writer `--author` names,
//! whose profile it makes when the site has none, or else the language's
//! only profile. Every post needs an author with a profile to publish
//! without a warning, and a new user should not have to know that first.

use std::io;
use std::path::Path;

use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Locale, Slug, UserMessage};

use crate::init::quoted;
use crate::new::write;
use crate::new_message;

/// The author `khalid new` chose for a post.
#[derive(Debug)]
pub(crate) enum Byline<'a> {
    /// The post names `slug`. `unlisted` is the name to make a profile for
    /// when the language has no profile with that slug.
    Named {
        slug: Slug,
        unlisted: Option<&'a str>,
    },
    /// No author could be chosen: `profiles` lists the language's profiles
    /// to pick from, empty when it has none.
    Open { profiles: Vec<Slug> },
}

impl Byline<'_> {
    /// The slug the post names, if any.
    pub(crate) fn slug(&self) -> Option<&Slug> {
        match self {
            Self::Named { slug, .. } => Some(slug),
            Self::Open { .. } => None,
        }
    }
}

/// The author of a new post in `locale` on the site in `site`: `name` from
/// `--author`, or the language's only profile. Reads files, writes none.
///
/// # Errors
///
/// A [`UserMessage`] when `name` has nothing to make a slug from.
pub(crate) fn choose<'a>(
    site: &Path,
    config: &Config,
    locale: &Locale,
    name: Option<&'a str>,
) -> Result<Byline<'a>, UserMessage> {
    let profiles = profiles(site, config, locale);
    let Some(name) = name.map(str::trim) else {
        return Ok(match profiles.as_slice() {
            [only] => Byline::Named {
                slug: only.clone(),
                unlisted: None,
            },
            _ => Byline::Open { profiles },
        });
    };
    let slug = Slug::from_title(name, locale, config.slugs())
        .map_err(|_| new_message::no_author_slug(name))?;
    let unlisted = (!profiles.contains(&slug)).then_some(name);
    Ok(Byline::Named { slug, unlisted })
}

/// Writes the profile `byline` needs in `locale`, if any: its name and slug
/// in `content/<locale>/authors/<slug>.md`. Returns that path, relative to
/// the site, or `None` when nothing was written. A file already there is
/// the writer's, even with another slug inside, and is never replaced.
///
/// # Errors
///
/// A [`UserMessage`] when the file cannot be written.
pub(crate) fn make_profile(
    site: &Path,
    locale: &Locale,
    byline: &Byline<'_>,
) -> Result<Option<String>, UserMessage> {
    let Byline::Named {
        slug,
        unlisted: Some(name),
    } = byline
    else {
        return Ok(None);
    };
    let folder = format!("content/{locale}/authors");
    let file = format!("{slug}.md");
    let relative = format!("{folder}/{file}");
    let text = format!(
        "---\nname: {}\nslug: {}\n---\n",
        quoted(name),
        quoted(slug.as_str())
    );
    match write(site, &folder, &file, &text) {
        Ok(()) => Ok(Some(relative)),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(None),
        Err(error) => Err(new_message::write(&relative, &error)),
    }
}

/// The slugs of the author profiles in `locale`, as `khalid build` names
/// their pages. Empty when the site's files cannot be read: the linter and
/// the build name the problem.
fn profiles(site: &Path, config: &Config, locale: &Locale) -> Vec<Slug> {
    let Ok(content) = ContentTree::walk(site).and_then(|tree| read_content(&tree)) else {
        return Vec::new();
    };
    content
        .authors
        .iter()
        .filter(|author| &author.locale == locale)
        .filter_map(|author| match (&author.front.slug, &author.front.name) {
            (Some(slug), _) => Some(slug.clone()),
            (None, Some(name)) => Slug::from_title(name, locale, config.slugs()).ok(),
            (None, None) => None,
        })
        .collect()
}
