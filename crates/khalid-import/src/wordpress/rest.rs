//! Reading a `WordPress` site through its public REST API, `/wp-json/`,
//! into the same [`Export`] a WXR file gives, so the plan, the map and the
//! report work the same. Only what a visitor may read comes back:
//! published posts and pages, their authors' public names, categories and
//! tags. Yoast's `yoast_head_json`, when the site has it, carries the
//! search title, description, canonical and robots settings.

use chrono::NaiveDateTime;
use khalid_core::AbsoluteUrl;
use serde::Deserialize;

use crate::wordpress::{Author, Export, Item, ItemKind, ItemStatus, Meta, Site, Term};

/// The JSON a site's REST API answered, one text per page of each list.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct RestAnswers {
    /// `/wp-json/`, which names the site.
    pub root: String,
    /// The pages of `/wp-json/wp/v2/posts`.
    pub posts: Vec<String>,
    /// The pages of `/wp-json/wp/v2/pages`.
    pub pages: Vec<String>,
    /// The pages of `/wp-json/wp/v2/users`.
    pub users: Vec<String>,
    /// The pages of `/wp-json/wp/v2/categories`.
    pub categories: Vec<String>,
    /// The pages of `/wp-json/wp/v2/tags`.
    pub tags: Vec<String>,
}

/// Why [`read_rest`] refused an answer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RestError {
    /// An answer is not the JSON `WordPress` gives there.
    #[error("the answer for {list} is not what WordPress gives there: {source}")]
    Json {
        /// Which list, such as `posts`.
        list: &'static str,
        /// What the reader said.
        #[source]
        source: serde_json::Error,
    },
    /// The site names a home address that is not one.
    #[error("the site's home address {0:?} is not an address")]
    Address(String),
}

#[derive(Deserialize)]
struct Root {
    name: String,
    #[serde(default)]
    home: Option<String>,
    url: String,
}

#[derive(Deserialize)]
struct Rendered {
    rendered: String,
}

#[derive(Deserialize)]
struct RestItem {
    id: u64,
    #[serde(default)]
    date: Option<String>,
    slug: String,
    status: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    link: Option<String>,
    title: Rendered,
    content: Rendered,
    #[serde(default)]
    excerpt: Option<Rendered>,
    #[serde(default)]
    author: u64,
    #[serde(default)]
    sticky: bool,
    #[serde(default)]
    parent: u64,
    #[serde(default)]
    categories: Vec<u64>,
    #[serde(default)]
    tags: Vec<u64>,
    #[serde(default)]
    yoast_head_json: Option<Yoast>,
}

#[derive(Deserialize)]
struct Yoast {
    title: Option<String>,
    description: Option<String>,
    canonical: Option<String>,
    robots: Option<YoastRobots>,
}

#[derive(Deserialize)]
struct YoastRobots {
    index: Option<String>,
    follow: Option<String>,
}

#[derive(Deserialize)]
struct RestUser {
    id: u64,
    name: String,
    slug: String,
}

#[derive(Deserialize)]
struct RestTerm {
    id: u64,
    name: String,
    slug: String,
    #[serde(default)]
    parent: u64,
}

/// The rows of every page of one list.
fn rows<T: for<'de> Deserialize<'de>>(
    list: &'static str,
    pages: &[String],
) -> Result<Vec<T>, RestError> {
    let mut rows = Vec::new();
    for page in pages {
        let read: Vec<T> =
            serde_json::from_str(page).map_err(|source| RestError::Json { list, source })?;
        rows.extend(read);
    }
    Ok(rows)
}

/// Reads what a site's REST API answered as an [`Export`]: posts first,
/// then pages, each list in the order the site gave it.
///
/// # Errors
///
/// Returns [`RestError::Json`] when an answer is not the JSON `WordPress`
/// gives, and [`RestError::Address`] when the site's home is not an address.
pub fn read_rest(answers: &RestAnswers) -> Result<Export, RestError> {
    let root: Root = serde_json::from_str(&answers.root).map_err(|source| RestError::Json {
        list: "the site",
        source,
    })?;
    let home = root.home.unwrap_or(root.url);
    let address = AbsoluteUrl::parse(&home).map_err(|_| RestError::Address(home.clone()))?;
    let users: Vec<RestUser> = rows("users", &answers.users)?;
    let categories: Vec<RestTerm> = rows("categories", &answers.categories)?;
    let tags: Vec<RestTerm> = rows("tags", &answers.tags)?;
    let mut items: Vec<RestItem> = rows("posts", &answers.posts)?;
    items.extend(rows::<RestItem>("pages", &answers.pages)?);
    let slug_of =
        |terms: &[RestTerm], id: u64| terms.iter().find(|t| t.id == id).map(|t| t.slug.clone());
    let term = |terms: &[RestTerm], term: &RestTerm| Term {
        id: Some(term.id),
        slug: term.slug.clone(),
        name: characters(&term.name),
        parent: Some(term.parent)
            .filter(|id| *id != 0)
            .and_then(|id| slug_of(terms, id)),
    };
    Ok(Export {
        site: Site {
            title: characters(&root.name),
            address,
            language: None,
        },
        authors: users
            .iter()
            .map(|user| Author {
                login: user.slug.clone(),
                display_name: characters(&user.name),
            })
            .collect(),
        categories: categories.iter().map(|t| term(&categories, t)).collect(),
        tags: tags.iter().map(|t| term(&tags, t)).collect(),
        items: items
            .into_iter()
            .map(|item| {
                let creator = users
                    .iter()
                    .find(|u| u.id == item.author)
                    .map(|u| u.slug.clone());
                let categories = item
                    .categories
                    .iter()
                    .filter_map(|id| slug_of(&categories, *id))
                    .collect();
                let tags = item
                    .tags
                    .iter()
                    .filter_map(|id| slug_of(&tags, *id))
                    .collect();
                item_of(item, creator.unwrap_or_default(), categories, tags)
            })
            .collect(),
    })
}

/// One post or page as an export item, with its creator's and terms'
/// slugs looked up already.
fn item_of(item: RestItem, creator: String, categories: Vec<String>, tags: Vec<String>) -> Item {
    let published = item
        .date
        .as_deref()
        .and_then(|date| NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S").ok());
    Item {
        id: item.id,
        kind: ItemKind::from_name(&item.kind),
        status: ItemStatus::from_name(&item.status),
        title: characters(&item.title.rendered),
        name: item.slug,
        link: item.link.and_then(|link| AbsoluteUrl::parse(&link).ok()),
        creator,
        published,
        content: item.content.rendered,
        excerpt: item
            .excerpt
            .map(|e| characters(&e.rendered))
            .unwrap_or_default(),
        sticky: item.sticky,
        parent: Some(item.parent).filter(|id| *id != 0),
        attachment_url: None,
        categories,
        tags,
        meta: item.yoast_head_json.map(yoast_meta).unwrap_or_default(),
    }
}

/// Yoast's head as the custom fields a WXR file holds, so the SEO reader
/// finds them where it looks.
fn yoast_meta(yoast: Yoast) -> Vec<Meta> {
    let robots = yoast.robots.unwrap_or(YoastRobots {
        index: None,
        follow: None,
    });
    let flag =
        |value: Option<String>, off: &str| (value.as_deref() == Some(off)).then(|| "1".to_owned());
    [
        ("_yoast_wpseo_title", yoast.title.map(|t| characters(&t))),
        (
            "_yoast_wpseo_metadesc",
            yoast.description.map(|t| characters(&t)),
        ),
        ("_yoast_wpseo_canonical", yoast.canonical),
        (
            "_yoast_wpseo_meta-robots-noindex",
            flag(robots.index, "noindex"),
        ),
        (
            "_yoast_wpseo_meta-robots-nofollow",
            flag(robots.follow, "nofollow"),
        ),
    ]
    .into_iter()
    .filter_map(|(key, value)| {
        Some(Meta {
            key: key.to_owned(),
            value: value?,
        })
    })
    .collect()
}

/// `text` with numeric character references such as `&#8217;` written as
/// the characters they stand for; named ones are left for
/// `names::decoded`, as in a WXR file.
fn characters(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, after)) = rest.split_once("&#") {
        out.push_str(before);
        let decoded = after.split_once(';').and_then(|(code, tail)| {
            let number = match code.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => code.parse().ok(),
            };
            Some((char::from_u32(number?)?, tail))
        });
        if let Some((c, tail)) = decoded {
            out.push(c);
            rest = tail;
        } else {
            out.push_str("&#");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}
