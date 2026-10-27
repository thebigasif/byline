//! Reading the JSON file that Settings, Import/Export in Ghost writes,
//! including the older layouts of Ghost 0.x to 3.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::ghost::{
    Author, Body, CodeInjection, Export, Post, PostKind, PostStatus, Seo, Tag, Visibility,
};

/// Why [`read_export`] refused a file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExportError {
    /// The file is not JSON, often because the download stopped part of the
    /// way through, or a field holds JSON of the wrong kind.
    #[error("the file is not JSON that Ghost writes, at line {line}, column {column}: {source}")]
    NotJson {
        /// The file's line, counting from 1.
        line: usize,
        /// The column on that line, counting from 1.
        column: usize,
        /// What the JSON reader found.
        #[source]
        source: serde_json::Error,
    },
    /// The file is JSON, but has no `db` list with the data of a Ghost
    /// export in it.
    #[error("the file is not a Ghost export: it has no db list with data in it")]
    NotGhost,
    /// A field holds a value of the wrong kind, such as a date that is not a
    /// date.
    #[error("post {post}: {field} holds {value:?}, which is not what Ghost writes there")]
    BadValue {
        /// The post's id.
        post: String,
        /// The field, such as `published_at`.
        field: &'static str,
        /// The value as written.
        value: String,
    },
}

/// The file, which wraps the database in a `db` list.
#[derive(Deserialize)]
struct File {
    #[serde(default)]
    db: Vec<Database>,
}

#[derive(Deserialize)]
struct Database {
    meta: Option<Meta>,
    data: Data,
}

#[derive(Deserialize)]
struct Meta {
    version: Option<String>,
}

#[derive(Deserialize)]
struct Data {
    #[serde(default)]
    posts: Vec<RawPost>,
    #[serde(default)]
    posts_meta: Vec<RawMeta>,
    #[serde(default)]
    users: Vec<RawUser>,
    #[serde(default)]
    tags: Vec<RawTag>,
    #[serde(default)]
    posts_authors: Vec<RawLink>,
    #[serde(default)]
    posts_tags: Vec<RawLink>,
    #[serde(default)]
    settings: Vec<Setting>,
}

#[derive(Deserialize)]
struct RawUser {
    #[serde(deserialize_with = "id")]
    id: String,
    name: Option<String>,
    slug: Option<String>,
    bio: Option<String>,
    website: Option<String>,
    twitter: Option<String>,
    facebook: Option<String>,
}

#[derive(Deserialize)]
struct RawTag {
    #[serde(deserialize_with = "id")]
    id: String,
    name: Option<String>,
    slug: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
}

/// A row of `posts_authors` or `posts_tags`: a post and what it links to.
#[derive(Deserialize)]
struct RawLink {
    #[serde(deserialize_with = "id")]
    post_id: String,
    #[serde(alias = "author_id", alias = "tag_id", deserialize_with = "id")]
    to: String,
    sort_order: Option<u64>,
}

#[derive(Deserialize)]
struct RawPost {
    #[serde(deserialize_with = "id")]
    id: String,
    title: Option<String>,
    slug: Option<String>,
    lexical: Option<String>,
    mobiledoc: Option<String>,
    html: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    /// Before Ghost 2, a page was a post with `page` set.
    #[serde(default, deserialize_with = "flag")]
    page: bool,
    /// Before Ghost 1, a post had one author, named on the post itself.
    #[serde(default, deserialize_with = "maybe_id")]
    author_id: Option<String>,
    status: Option<String>,
    visibility: Option<String>,
    published_at: Option<Value>,
    custom_excerpt: Option<String>,
    feature_image: Option<String>,
    #[serde(default, deserialize_with = "flag")]
    featured: bool,
    canonical_url: Option<String>,
    codeinjection_head: Option<String>,
    codeinjection_foot: Option<String>,
    /// Before Ghost 4, the SEO fields sat on the post itself.
    #[serde(flatten)]
    seo: RawSeo,
}

/// One row of `posts_meta`, where Ghost 4 and later keep the SEO fields.
#[derive(Deserialize)]
struct RawMeta {
    #[serde(deserialize_with = "id")]
    post_id: String,
    #[serde(flatten)]
    seo: RawSeo,
}

#[derive(Deserialize, Default)]
struct RawSeo {
    meta_title: Option<String>,
    meta_description: Option<String>,
    og_title: Option<String>,
    og_description: Option<String>,
    og_image: Option<String>,
    twitter_title: Option<String>,
    twitter_description: Option<String>,
    twitter_image: Option<String>,
}

#[derive(Deserialize)]
struct Setting {
    key: String,
    value: Option<Value>,
}

/// Reads a Ghost export file into an [`Export`].
///
/// # Errors
///
/// Returns [`ExportError::NotJson`] with the place when the text is not
/// JSON of the shape Ghost writes, [`ExportError::NotGhost`] when it is not
/// a Ghost export, and [`ExportError::BadValue`] when a post's date is not
/// a date.
pub fn read_export(file: &str) -> Result<Export, ExportError> {
    let file: File = serde_json::from_str(file).map_err(|source| ExportError::NotJson {
        line: source.line(),
        column: source.column(),
        source,
    })?;
    let Database { meta, data } = file.db.into_iter().next().ok_or(ExportError::NotGhost)?;
    let mut seo: BTreeMap<String, RawSeo> = data
        .posts_meta
        .into_iter()
        .map(|row| (row.post_id, row.seo))
        .collect();
    let (mut authors, mut tags) = (linked(data.posts_authors), linked(data.posts_tags));
    let posts = data
        .posts
        .into_iter()
        .map(|raw| {
            let links = Links {
                seo: seo.remove(&raw.id).unwrap_or_default(),
                authors: authors.remove(&raw.id).unwrap_or_default(),
                tags: tags.remove(&raw.id).unwrap_or_default(),
            };
            post(raw, links)
        })
        .collect::<Result<_, _>>()?;
    let title = data
        .settings
        .into_iter()
        .find(|setting| setting.key == "title")
        .and_then(|setting| blank(setting.value?.as_str().map(str::to_owned)));
    Ok(Export {
        title,
        version: meta.and_then(|meta| meta.version),
        posts,
        authors: data.users.into_iter().map(author).collect(),
        tags: data.tags.into_iter().map(tag).collect(),
    })
}

/// What the export's other tables say about one post.
struct Links {
    /// Its `posts_meta` row.
    seo: RawSeo,
    /// The ids of its authors, in order.
    authors: Vec<String>,
    /// The ids of its tags, in order.
    tags: Vec<String>,
}

/// The ids each post links to in `links`, by the post's id, in each link's
/// `sort_order` and then in file order.
fn linked(mut links: Vec<RawLink>) -> BTreeMap<String, Vec<String>> {
    links.sort_by_key(|link| link.sort_order.unwrap_or(0));
    let mut by_post: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for link in links {
        by_post.entry(link.post_id).or_default().push(link.to);
    }
    by_post
}

fn author(user: RawUser) -> Author {
    Author {
        id: user.id,
        name: user.name.unwrap_or_default(),
        slug: user.slug.unwrap_or_default(),
        bio: blank(user.bio),
        website: blank(user.website),
        twitter: blank(user.twitter),
        facebook: blank(user.facebook),
    }
}

fn tag(raw: RawTag) -> Tag {
    let name = raw.name.unwrap_or_default();
    Tag {
        internal: raw.visibility.as_deref() == Some("internal") || name.starts_with('#'),
        id: raw.id,
        slug: raw.slug.unwrap_or_default(),
        description: blank(raw.description),
        name,
    }
}

/// One post from its row and the other tables. A `posts_meta` field wins
/// over the post's own unless it is blank, and a post with no
/// `posts_authors` rows keeps the one author older exports name on it.
fn post(raw: RawPost, links: Links) -> Result<Post, ExportError> {
    let published = time(&raw.id, raw.published_at)?;
    let pick = |own: Option<String>, from_meta: Option<String>| blank(from_meta).or(blank(own));
    let Links {
        seo: meta,
        mut authors,
        tags,
    } = links;
    if authors.is_empty() {
        authors.extend(raw.author_id);
    }
    let own = raw.seo;
    let seo = Seo {
        meta_title: pick(own.meta_title, meta.meta_title),
        meta_description: pick(own.meta_description, meta.meta_description),
        og_title: pick(own.og_title, meta.og_title),
        og_description: pick(own.og_description, meta.og_description),
        og_image: pick(own.og_image, meta.og_image),
        twitter_title: pick(own.twitter_title, meta.twitter_title),
        twitter_description: pick(own.twitter_description, meta.twitter_description),
        twitter_image: pick(own.twitter_image, meta.twitter_image),
    };
    let body = match (blank(raw.lexical), blank(raw.mobiledoc), blank(raw.html)) {
        (Some(lexical), _, _) => Body::Lexical(lexical),
        (None, Some(mobiledoc), _) => Body::Mobiledoc(mobiledoc),
        (None, None, Some(html)) => Body::Html(html),
        (None, None, None) => Body::Empty,
    };
    let kind = if raw.page || raw.kind.as_deref() == Some("page") {
        PostKind::Page
    } else {
        PostKind::Post
    };
    Ok(Post {
        kind,
        status: PostStatus::from_name(raw.status.as_deref().unwrap_or("draft")),
        visibility: Visibility::from_name(raw.visibility.as_deref().unwrap_or("public")),
        title: raw.title.unwrap_or_default(),
        slug: raw.slug.unwrap_or_default(),
        published,
        body,
        excerpt: blank(raw.custom_excerpt),
        feature_image: blank(raw.feature_image),
        featured: raw.featured,
        canonical_url: blank(raw.canonical_url),
        seo,
        code_injection: CodeInjection {
            head: blank(raw.codeinjection_head),
            foot: blank(raw.codeinjection_foot),
        },
        authors,
        tags,
        id: raw.id,
    })
}

/// `value`, unless it is blank.
fn blank(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

/// A post's date: text such as `2024-03-12T09:30:00.000Z`, or, in old
/// exports, milliseconds since 1970. Ghost keeps dates in UTC.
fn time(post: &str, value: Option<Value>) -> Result<Option<NaiveDateTime>, ExportError> {
    let bad = |value: String| ExportError::BadValue {
        post: post.to_owned(),
        field: "published_at",
        value,
    };
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let bare = text.trim().trim_end_matches('Z');
            match NaiveDateTime::parse_from_str(bare, "%Y-%m-%dT%H:%M:%S%.f") {
                Ok(at) => Ok(Some(at)),
                Err(_) => Err(bad(text)),
            }
        }
        Some(Value::Number(number)) => number
            .as_i64()
            .and_then(DateTime::from_timestamp_millis)
            .map(|at| Some(at.naive_utc()))
            .ok_or_else(|| bad(number.to_string())),
        Some(other) => Err(bad(other.to_string())),
    }
}

/// An id, which Ghost 1 and later write as text and older versions as a
/// number.
#[derive(Deserialize)]
#[serde(untagged)]
enum Id {
    Text(String),
    Number(u64),
}

impl From<Id> for String {
    fn from(id: Id) -> Self {
        match id {
            Id::Text(text) => text,
            Id::Number(number) => number.to_string(),
        }
    }
}

fn id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Id::deserialize(deserializer).map(String::from)
}

/// An id that may be missing or null.
fn maybe_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<Id>::deserialize(deserializer).map(|id| id.map(String::from))
}

/// A yes or no, which exports from a SQLite database may write as 0 or 1,
/// and leave as null for no.
fn flag<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Flag {
        Yes(bool),
        Number(u64),
    }
    Ok(match Option::<Flag>::deserialize(deserializer)? {
        Some(Flag::Yes(yes)) => yes,
        Some(Flag::Number(number)) => number != 0,
        None => false,
    })
}
