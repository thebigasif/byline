//! A post: its front matter and its markdown body.

use std::path::Path;

use chrono::NaiveDate;
use khalid_core::{
    AbsoluteUrl, HeadCode, PasswordHash, PostAccess, RobotsDirectives, Slug, TemplateName,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use crate::front_matter::{Fence, split};
use crate::post_fields::{self, LeadImage, ReviewState, ShareCards, Sitemap, TopicCluster};
use crate::{ContentError, parse_blocks};

/// What a post's front matter may say.
///
/// Title, date and author may be missing: the linter blocks a post without
/// them and explains why, which teaches more than a refusal to read the file.
/// Unknown fields and values of the wrong kind are refused here.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PostFront {
    /// The headline.
    pub title: Option<String>,
    /// The last part of the address. Made from the title when missing.
    pub slug: Option<Slug>,
    /// The day the post is dated.
    #[serde(default, deserialize_with = "date")]
    pub date: Option<NaiveDate>,
    /// The day of the last real update, which must be the day the change
    /// happened, never the day it was saved. Feeds the visible date,
    /// `dateModified`, the sitemap's `lastmod` and the stale-post rule.
    /// A title or description edit alone must never set it (F1).
    #[serde(default, deserialize_with = "date")]
    pub updated: Option<NaiveDate>,
    /// What changed in the last real update, in one plain sentence, such
    /// as "Refreshed the 2024 numbers." Shown beside the updated date.
    pub updated_note: Option<String>,
    /// The day the post last came up for review and was looked at (F3,
    /// decision D4: at least every six months). A review that changes
    /// nothing sets this and nothing else: it never moves `date` or
    /// `updated`, because a reader seeing a new date expects a real
    /// change.
    pub reviewed: Option<NaiveDate>,
    /// The topic group the post belongs to, shared by every post in it,
    /// such as `internal-links` (P8). A group whose main post says
    /// `main_post: true` is checked by the linter's cluster rules.
    pub topic_group: Option<String>,
    /// Whether this post is its topic group's main post, the pillar the
    /// group's other posts should link to (P8).
    #[serde(default)]
    pub main_post: bool,
    /// The author's slug, matching a file in `authors/`.
    pub author: Option<Slug>,
    /// The category's slug, matching one in `categories.toml`.
    pub category: Option<Slug>,
    /// The summary for search results and feeds.
    pub description: Option<String>,
    /// The key shared by the versions of this post in other languages.
    pub translation: Option<String>,
    /// Old paths that redirect here, such as `/2019/05/old-title/`, so a
    /// moved post keeps its links and rankings.
    #[serde(default, deserialize_with = "aliases")]
    pub aliases: Vec<String>,
    /// Whether the post is featured, for the featured channel and lists.
    #[serde(default)]
    pub featured: bool,
    /// The post's own robots directives, such as `noindex, nofollow`, in
    /// place of the site's defaults.
    pub robots: Option<RobotsDirectives>,
    /// The address search engines should credit instead of this post, such
    /// as the original of a post first published elsewhere.
    pub canonical: Option<AbsoluteUrl>,
    /// The slugs of the authors who checked the post, shown as "Reviewed
    /// by" and written as `reviewedBy` in the page's JSON-LD.
    #[serde(default, deserialize_with = "slugs")]
    pub reviewed_by: Vec<Slug>,
    /// The slugs of the posts shown under this one, in place of the ones
    /// chosen at build time. At most [`MAX_RELATED`].
    #[serde(default, deserialize_with = "related")]
    pub related: Vec<Slug>,
    /// The phrase the post wants to be found for in search, such as
    /// "internal links". The linter checks that the title, description,
    /// opening and other key places hold it.
    #[serde(default, deserialize_with = "focus_keyword")]
    pub focus_keyword: Option<String>,
    /// Other phrases close to the focus keyword that the post also covers,
    /// such as "link text". At most [`MAX_SECONDARY_KEYWORDS`].
    #[serde(default, deserialize_with = "secondary_keywords")]
    pub secondary_keywords: Vec<String>,
    /// The topic cluster the post belongs to and its role in it, which the
    /// linter checks: a member links to its pillar.
    pub cluster: Option<TopicCluster>,
    /// The image at the top of the post and on its cards.
    pub image: Option<LeadImage>,
    /// The title for search results and the browser tab, in place of the
    /// headline, such as "Internal links, a guide for 2026".
    #[serde(default, deserialize_with = "post_fields::seo_title")]
    pub seo_title: Option<String>,
    /// The tags' names, such as "internal links", in the order written.
    #[serde(default, deserialize_with = "post_fields::tags")]
    pub tags: Vec<String>,
    /// Whether the post is listed in the XML sitemap.
    #[serde(default)]
    pub sitemap: Sitemap,
    /// Who may read the body: `public`, the default, or `members`. Anyone
    /// else sees the title, the description and a note.
    #[serde(default)]
    pub access: PostAccess,
    /// A theme template to show the post with, such as `wide` for
    /// `theme/templates/post-wide.html`, in place of `post.html`.
    pub template: Option<TemplateName>,
    /// The Argon2id hash of the password a reader must give to read the
    /// body. Set through the Admin API, which never writes the password.
    pub password_hash: Option<PasswordHash>,
    /// Checked HTML for this post's page head, such as an analytics tag.
    pub head_code: Option<HeadCode>,
    /// The other authors' slugs, shown after the author in the byline.
    #[serde(default, deserialize_with = "slugs")]
    pub co_authors: Vec<Slug>,
    /// Where the post stands in review.
    #[serde(default)]
    pub review_state: ReviewState,
    /// How the post looks when shared on X and Facebook.
    #[serde(default)]
    pub share_cards: ShareCards,
    /// Hand-written JSON-LD for a type no block writes, such as an `Event`
    /// or a `Course`, added to the page's structured data.
    #[serde(default, deserialize_with = "post_fields::schema")]
    pub schema: Option<String>,
}

/// The most related posts a post shows, and so the most it may pin.
pub const MAX_RELATED: usize = 5;

/// The most secondary keywords a post may name. Five keywords in all,
/// with the focus keyword, as Rank Math allows in its free version.
pub const MAX_SECONDARY_KEYWORDS: usize = 4;

/// A post as read from its file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Post {
    /// The front matter.
    pub front: PostFront,
    /// The markdown after the front matter, exactly as written.
    pub body: String,
}

/// Reads a post from `text`, naming `path` in errors.
///
/// # Errors
///
/// Returns a [`ContentError`] when the front matter fence is missing or never
/// closed, when a field is unknown, has the wrong kind of value, or the YAML
/// is broken, or when a block in the body is wrong. Field and block errors
/// carry the file's line.
pub fn parse_post(path: &Path, text: &str) -> Result<Post, ContentError> {
    let (front, body) = parse_front(path, text)?;
    Ok(Post { front, body })
}

/// Reads any kind of front matter from `text`, naming `path` in errors, and
/// returns it with the body exactly as written once its blocks are checked.
pub(crate) fn parse_front<F: DeserializeOwned + Default>(
    path: &Path,
    text: &str,
) -> Result<(F, String), ContentError> {
    let parts = split(text).map_err(|fence| match fence {
        Fence::Missing => ContentError::NoFrontMatter {
            path: path.to_path_buf(),
        },
        Fence::Unclosed => ContentError::UnclosedFrontMatter {
            path: path.to_path_buf(),
        },
    })?;
    let front = if parts.yaml.trim().is_empty() {
        F::default()
    } else {
        serde_saphyr::from_str(parts.yaml).map_err(|source| located(path, parts.yaml, source))?
    };
    parse_blocks(parts.body).map_err(|source| {
        // The body starts on the line after the closing ---.
        let before = text
            .get(..text.len().saturating_sub(parts.body.len()))
            .unwrap_or_default()
            .matches('\n')
            .count();
        ContentError::Block {
            path: path.to_path_buf(),
            line: before.saturating_add(source.line()),
            source,
        }
    })?;
    Ok((front, parts.body.to_owned()))
}

/// Turns a YAML error into one that names the file's line and field.
fn located(path: &Path, yaml: &str, source: serde_saphyr::Error) -> ContentError {
    let yaml_line = source
        .location()
        .and_then(|location| usize::try_from(location.line()).ok())
        .unwrap_or(1);
    // The key on the error's line, or, for an item of a list such as
    // `  - old-title` or a key inside a group such as `  alt:`, the nearest
    // key above it that starts a line.
    let lines: Vec<&str> = yaml.lines().take(yaml_line).collect();
    let own = lines
        .last()
        .and_then(|line| line.split_once(':'))
        .map(|(key, _)| key);
    let above = lines
        .iter()
        .rev()
        .skip(1)
        .find(|line| !line.starts_with([' ', '-']) && line.contains(':'))
        .and_then(|line| line.split_once(':'));
    let in_group = above.is_some_and(|(_, value)| value.trim().is_empty());
    let field = own
        .filter(|key| !key.trim_start().starts_with('-'))
        .filter(|key| !(in_group && key.starts_with(' ')))
        .or_else(|| {
            lines
                .iter()
                .rev()
                .find(|line| !line.starts_with([' ', '-']) && line.contains(':'))
                .and_then(|line| line.split_once(':'))
                .map(|(key, _)| key)
        })
        .map_or_else(String::new, |key| key.trim().to_owned());
    let rendered = source.to_string();
    let first = rendered.lines().next().unwrap_or_default();
    // The reader writes "error: line 2 column 7: <problem>"; keep the problem.
    let without_error = first.strip_prefix("error: ").unwrap_or(first);
    let problem = match without_error.split_once(": ") {
        Some((place, problem)) if place.starts_with("line ") => problem,
        _ => without_error,
    }
    .to_owned();
    // Two of the reader's phrases mean nothing to a writer; say what happened.
    let problem = match problem.as_str() {
        "expected string scalar" => {
            "this field must be plain text, not a list or a group".to_owned()
        }
        "mapping values are not allowed in this context" => {
            "the line has a second colon; put the value in quotes if it needs one".to_owned()
        }
        _ => problem,
    };
    ContentError::Field {
        path: path.to_path_buf(),
        // The YAML starts on the file's second line, after the opening ---.
        line: yaml_line.saturating_add(1),
        field,
        problem,
        source: Box::new(source),
    }
}

/// Reads a date written as `YYYY-MM-DD`, with a message a writer understands.
pub(crate) fn date<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<NaiveDate>, D::Error> {
    let text = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&text, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| {
            serde::de::Error::custom(
                "the date must be a real day written as YYYY-MM-DD, such as 2026-09-14",
            )
        })
}

/// One slug or a list of them, as a writer may write either.
#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

/// Reads one text or a list of them, as a list.
pub(crate) fn one_or_many<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(text) => vec![text],
        OneOrMany::Many(texts) => texts,
    })
}

/// Reads one slug or a list of slugs, with a message a writer understands.
fn slugs<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Slug>, D::Error> {
    one_or_many(deserializer)?
        .iter()
        .map(|text| {
            Slug::try_new(text).map_err(|_| {
                serde::de::Error::custom(format!(
                    "{text:?} is not a slug; name the file in authors/ or posts/ without .md, such as lena-vogt"
                ))
            })
        })
        .collect()
}

/// Reads the pinned related posts: slugs, at most [`MAX_RELATED`].
fn related<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Slug>, D::Error> {
    let slugs = slugs(deserializer)?;
    if slugs.len() > MAX_RELATED {
        return Err(serde::de::Error::custom(format!(
            "a post shows at most {MAX_RELATED} related posts, and this lists {}; keep the {MAX_RELATED} that matter most",
            slugs.len()
        )));
    }
    Ok(slugs)
}

/// Refuses a keyword with no words in it, with a message a writer
/// understands.
fn keyword<E: serde::de::Error>(text: String) -> Result<String, E> {
    if text.trim().is_empty() {
        return Err(E::custom(
            "a keyword needs at least one word, such as internal links; remove the field if the post has none",
        ));
    }
    Ok(text)
}

/// Reads the focus keyword: one phrase with at least one word.
fn focus_keyword<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    keyword(String::deserialize(deserializer)?).map(Some)
}

/// Reads the secondary keywords: one phrase or a list, each with at least
/// one word, at most [`MAX_SECONDARY_KEYWORDS`].
fn secondary_keywords<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let texts = one_or_many(deserializer)?;
    if texts.len() > MAX_SECONDARY_KEYWORDS {
        return Err(serde::de::Error::custom(format!(
            "a post has at most {MAX_SECONDARY_KEYWORDS} secondary keywords, and this lists {}; keep the {MAX_SECONDARY_KEYWORDS} closest to the focus keyword",
            texts.len()
        )));
    }
    texts.into_iter().map(keyword).collect()
}

/// Reads old paths, each starting with `/`, with a message a writer
/// understands.
fn aliases<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let paths = Vec::<String>::deserialize(deserializer)?;
    match paths
        .iter()
        .find(|path| !path.starts_with('/') || path.contains(char::is_whitespace))
    {
        Some(bad) => Err(serde::de::Error::custom(format!(
            "{bad:?} is not a path; an alias is an old address on this site starting with /, such as /2019/05/old-title/"
        ))),
        None => Ok(paths),
    }
}
