//! One post, as its page shows it.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use minijinja::Value;
use serde::Serialize;

use crate::serde_display::{display, display_option};
use crate::{Contents, Person, PostCard, ReadingTime, RenderComments, RenderImage};

/// One post: its heading, dates, people, topics and body.
///
/// Exhaustive on purpose, like every input record (ADR 0009).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderPost {
    /// The heading, shown as the page's one `h1`.
    pub title: String,
    /// The day the post was published.
    #[serde(serialize_with = "display")]
    pub date: NaiveDate,
    /// The day it last changed in a way readers should know about.
    #[serde(serialize_with = "display_option")]
    pub updated: Option<NaiveDate>,
    /// How long it takes to read, from [`ReadingTime::of`] its body.
    pub reading_time: ReadingTime,
    /// Who wrote it, first author first.
    pub authors: Vec<Person>,
    /// Who checked it, from `reviewed_by` in its front matter, linked to
    /// their author pages.
    pub reviewers: Vec<Link>,
    /// The one category it belongs to, if any.
    pub category: Option<Link>,
    /// Its tags, in the order the writer gave them.
    pub tags: Vec<Link>,
    /// The lead image under the heading, usually loaded at once.
    pub image: Option<RenderImage>,
    /// The table of contents, from [`Contents::of`] the body's headings.
    pub contents: Option<Contents>,
    /// For each body image drawn smaller than its original, its address as
    /// the body writes it and the address of its largest version, which
    /// the image links to.
    pub originals: BTreeMap<String, String>,
    /// The posts shown under this one, from [`related`](crate::related).
    pub related: Vec<PostCard>,
    /// The site's posts whose bodies link to this one, newest first.
    pub mentions: Vec<Link>,
    /// The readers' comments and the form to leave one, or `None` where
    /// the site takes no comments, such as a page built without a server.
    pub comments: Option<RenderComments>,
    /// The post's HTML, already sanitised.
    pub body: BodyHtml,
}

/// A named link to a page on this site: an author, a tag or a category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Link {
    /// The name a reader sees.
    pub name: String,
    /// The page's path, such as `/blog/authors/asif/`.
    pub href: String,
}

/// HTML that was sanitised before it reached this crate, so templates print
/// it as it is (RULES.md BL-SEC-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyHtml(String);

impl BodyHtml {
    /// Wraps HTML that the content crate already sanitised with `ammonia`.
    /// Never pass text a person wrote straight into this.
    pub fn from_sanitised(html: String) -> Self {
        Self(html)
    }

    /// The HTML.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for BodyHtml {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Value::from_safe_string(self.0.clone()).serialize(serializer)
    }
}
