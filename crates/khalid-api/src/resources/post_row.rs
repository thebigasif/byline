//! A post as the admin's posts table lists it: the columns it shows and
//! the ones it filters by, without the body.

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_core::{Indexing, Slug};
use khalid_lint::{Report, Severity};
use serde::Serialize;
use utoipa::ToSchema;

use super::{AdminPost, Listable, PostStatus, ReviewState, display, display_option};
use crate::query::{FieldValues, Literal};

/// How many lint findings a post has, by severity. A post with any
/// `blocks` cannot be published.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct LintCounts {
    /// Findings that stop the post from going live.
    blocks: u32,
    /// Findings the writer should see, which do not stop it.
    warnings: u32,
}

impl LintCounts {
    /// `blocks` blocking findings and `warnings` warnings.
    pub fn new(blocks: u32, warnings: u32) -> Self {
        Self { blocks, warnings }
    }

    /// The counts in `report`.
    pub fn of(report: &Report) -> Self {
        let count = |severity| {
            let found = report
                .findings()
                .iter()
                .filter(|f| f.severity() == severity);
            u32::try_from(found.count()).unwrap_or(u32::MAX)
        };
        Self::new(count(Severity::Block), count(Severity::Warn))
    }

    /// Blocking findings.
    pub fn blocks(self) -> u32 {
        self.blocks
    }

    /// Warnings.
    pub fn warnings(self) -> u32 {
        self.warnings
    }
}

/// Whether search engines may list a post, as far as the site itself
/// says. Google's own answer comes with the Search Console connection
/// (plan section 5.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum IndexStatus {
    /// Live, and open to search engines.
    Indexable,
    /// Live, with robots set to `noindex`.
    Noindex,
    /// Live, with a canonical address that credits another page.
    CanonicalElsewhere,
    /// A draft or scheduled post, which search engines cannot reach.
    NotLive,
}

impl IndexStatus {
    /// What `post`'s status and settings say.
    pub fn of(post: &AdminPost) -> Self {
        let settings = post.settings();
        let noindex = |robots: &khalid_core::RobotsDirectives| robots.indexing == Indexing::NoIndex;
        if post.status() != PostStatus::Published {
            Self::NotLive
        } else if settings.robots.as_ref().is_some_and(noindex) {
            Self::Noindex
        } else if settings.canonical.is_some() {
            Self::CanonicalElsewhere
        } else {
            Self::Indexable
        }
    }

    /// The name the API uses, such as `not_live`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Indexable => "indexable",
            Self::Noindex => "noindex",
            Self::CanonicalElsewhere => "canonical_elsewhere",
            Self::NotLive => "not_live",
        }
    }
}

/// One row of the admin's posts table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PostRow {
    /// The post's identity, as in `/posts/{id}/`.
    id: String,
    /// The last part of its address.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "choosing-anchor-text")]
    slug: Slug,
    /// The title.
    title: String,
    /// The author's slug, if any.
    #[serde(serialize_with = "display_option")]
    #[schema(value_type = Option<String>, example = "asif")]
    author: Option<Slug>,
    /// Draft, published or scheduled.
    status: PostStatus,
    /// Draft, in review or ready.
    review_state: ReviewState,
    /// Whether the post is featured.
    featured: bool,
    /// When it went or goes live; empty for a draft.
    published_at: Option<DateTime<Utc>>,
    /// When it last changed.
    updated_at: DateTime<Utc>,
    /// Who made the last change.
    updated_by: Option<String>,
    /// Its lint findings, counted.
    lint: LintCounts,
    /// Whether search engines may list it.
    index_status: IndexStatus,
}

impl PostRow {
    /// The row for `post`, whose lint findings are `lint`.
    pub fn new(post: &AdminPost, lint: LintCounts) -> Self {
        Self {
            id: post.id().to_owned(),
            slug: post.slug().clone(),
            title: post.title().to_owned(),
            author: post.author().cloned(),
            status: post.status(),
            review_state: post.settings().review_state,
            featured: post.settings().featured,
            published_at: post.published_at(),
            updated_at: post.updated_at(),
            updated_by: post.updated_by().map(str::to_owned),
            lint,
            index_status: IndexStatus::of(post),
        }
    }

    /// The post's identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Its lint findings, counted.
    pub fn lint(&self) -> LintCounts {
        self.lint
    }

    /// Whether search engines may list it.
    pub fn index_status(&self) -> IndexStatus {
        self.index_status
    }
}

impl FieldValues for PostRow {
    fn values(&self, field: &str) -> Vec<Literal> {
        let text = |value: &str| vec![Literal::Text(value.to_owned())];
        let time = |at: &DateTime<Utc>| text(&at.to_rfc3339_opts(SecondsFormat::Secs, true));
        let count = |n: u32| vec![Literal::Integer(i64::from(n))];
        match field {
            "id" => text(&self.id),
            "slug" => text(self.slug.as_str()),
            "title" => text(&self.title),
            "author" => self.author.iter().flat_map(|a| text(a.as_str())).collect(),
            "status" => status_text(self.status),
            "review_state" => text(self.review_state.as_str()),
            "featured" => vec![Literal::Bool(self.featured)],
            "published_at" => self.published_at.iter().flat_map(time).collect(),
            "updated_at" => time(&self.updated_at),
            "lint.blocks" => count(self.lint.blocks),
            "lint.warnings" => count(self.lint.warnings),
            "index_status" => text(self.index_status.as_str()),
            _ => Vec::new(),
        }
    }
}

impl Listable for PostRow {
    const FIELDS: &'static [&'static str] = &[
        "id",
        "slug",
        "title",
        "author",
        "status",
        "review_state",
        "featured",
        "published_at",
        "updated_at",
        "lint.blocks",
        "lint.warnings",
        "index_status",
    ];

    fn id(&self) -> &str {
        &self.id
    }

    fn slug(&self) -> &Slug {
        &self.slug
    }
}

/// A status as the API writes it, such as `draft`, for a filter to match.
fn status_text(status: PostStatus) -> Vec<Literal> {
    let name = match status {
        PostStatus::Draft => "draft",
        PostStatus::Published => "published",
        PostStatus::Scheduled => "scheduled",
    };
    vec![Literal::Text(name.to_owned())]
}
