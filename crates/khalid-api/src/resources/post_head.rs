//! The SEO head of a post's page, for a front end that renders the post
//! itself (headless mode, KH-INS-5).

use std::sync::Arc;

use khalid_seo::{HeadTag, HeadTags, SchemaGraph};
use serde::{Serialize, Serializer};
use utoipa::ToSchema;

/// How the theme's page starts its JSON-LD, which [`PostHead::json_ld`]
/// leaves out.
const SCRIPT_OPEN: &str = "<script type=\"application/ld+json\">";

/// How it ends.
const SCRIPT_CLOSE: &str = "</script>\n";

/// What a post's page carries in its `<head>` for search engines and share
/// cards: the tags in the order the theme writes them, then the JSON-LD
/// graph. A front end that prints each tag in order, then the graph in a
/// `<script type="application/ld+json">` tag, has the theme's head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostHead(Arc<HeadParts>);

/// A post page's head: its tags in the order the theme writes them, then
/// its JSON-LD graph. A post holds them behind a shared pointer, so the
/// store copies no text.
#[derive(Debug, PartialEq, Eq, Serialize, ToSchema)]
pub struct HeadParts {
    /// The title, meta and link tags, in order.
    tags: Vec<HeadElement>,
    /// The schema graph as compact JSON, with each less-than sign escaped,
    /// so it can go inside a script tag as it is.
    #[schema(example = "{\"@context\":\"https://schema.org\",\"@graph\":[]}")]
    json_ld: String,
}

/// One tag of a post page's head. Values are plain text: a front end
/// escapes them as it writes the tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(tag = "tag", rename_all = "lowercase")]
#[non_exhaustive]
pub enum HeadElement {
    /// `<title>text</title>`.
    Title {
        /// The page title.
        text: String,
    },
    /// `<meta name="..." content="...">`, or with `property` for Open Graph.
    Meta {
        /// The `name` attribute, such as `description` or `twitter:card`.
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// The `property` attribute, such as `og:title`.
        #[serde(skip_serializing_if = "Option::is_none")]
        property: Option<String>,
        /// The `content` attribute.
        content: String,
    },
    /// `<link rel="..." hreflang="..." href="...">`, `hreflang` only on a
    /// language version.
    Link {
        /// The `rel` attribute, such as `canonical` or `alternate`.
        rel: String,
        /// The language of an `alternate` version, or `x-default`.
        #[serde(skip_serializing_if = "Option::is_none")]
        hreflang: Option<String>,
        /// The full address.
        href: String,
    },
}

impl PostHead {
    /// The head of a page with these SEO `tags` and schema `graph`, as
    /// `khalid-seo` writes them into the theme's page.
    pub fn new(tags: &HeadTags, graph: &SchemaGraph) -> Self {
        let script = graph.to_script_tag();
        let json_ld = script
            .strip_prefix(SCRIPT_OPEN)
            .and_then(|rest| rest.strip_suffix(SCRIPT_CLOSE))
            .unwrap_or_default()
            .to_owned();
        let tags = tags.as_slice().iter().filter_map(element).collect();
        Self(Arc::new(HeadParts { tags, json_ld }))
    }

    /// The tags, in order.
    pub fn tags(&self) -> &[HeadElement] {
        &self.0.tags
    }

    /// The schema graph as compact JSON, safe inside a script tag.
    pub fn json_ld(&self) -> &str {
        &self.0.json_ld
    }
}

impl Serialize for PostHead {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

/// `tag` in API shape. `None` for a kind of tag this version does not know.
fn element(tag: &HeadTag) -> Option<HeadElement> {
    let text = |value: &str| Some(value.to_owned());
    Some(match tag {
        HeadTag::Title(title) => HeadElement::Title {
            text: title.clone(),
        },
        HeadTag::Meta { name, content } => HeadElement::Meta {
            name: text(name),
            property: None,
            content: content.clone(),
        },
        HeadTag::Property { property, content } => HeadElement::Meta {
            name: None,
            property: text(property),
            content: content.clone(),
        },
        HeadTag::Link { rel, href } => HeadElement::Link {
            rel: (*rel).to_owned(),
            hreflang: None,
            href: href.as_str().to_owned(),
        },
        HeadTag::Alternate { hreflang, href } => HeadElement::Link {
            rel: "alternate".to_owned(),
            hreflang: Some(hreflang.clone()),
            href: href.as_str().to_owned(),
        },
        _ => return None,
    })
}
