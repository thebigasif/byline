//! The nodes every page carries: the publisher and the `WebSite`.

use khalid_core::AbsoluteUrl;

use crate::{
    PublisherKind, SchemaGraph, SchemaNode, SchemaValue, SeoError, SeoSite, ShareImage, UrlPath,
};

/// Whether the site has a search page, for the `WebSite`'s `SearchAction`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SiteSearch {
    /// No search, as in static mode.
    #[default]
    Off,
    /// Search results at this path, taking the words in `q`.
    On(UrlPath),
}

/// `base` with `#fragment`, the form every node id takes.
pub(crate) fn node_id(base: &AbsoluteUrl, fragment: &str) -> Result<AbsoluteUrl, SeoError> {
    AbsoluteUrl::parse(&format!("{}#{fragment}", base.as_str())).map_err(|source| SeoError::Url {
        path: format!("#{fragment}"),
        source,
    })
}

/// The site's home address in the default language.
pub(crate) fn site_home(site: &SeoSite) -> Result<AbsoluteUrl, SeoError> {
    site.url_for(&site.default_language, &UrlPath::root())
}

/// The `@id` of the publisher node, which every article points to.
pub(crate) fn publisher_id(site: &SeoSite) -> Result<AbsoluteUrl, SeoError> {
    let fragment = match site.publisher.kind {
        PublisherKind::Organization => "organization",
        PublisherKind::Person => "person",
    };
    node_id(&site_home(site)?, fragment)
}

/// An `ImageObject` for `image`.
pub(crate) fn image_node(image: &ShareImage) -> SchemaValue {
    let mut node = SchemaNode::new("ImageObject")
        .with("url", SchemaValue::Url(image.url.clone()))
        .with("width", SchemaValue::Integer(i64::from(image.width)))
        .with("height", SchemaValue::Integer(i64::from(image.height)));
    if !image.alt.is_empty() {
        node.set("caption", SchemaValue::text(&image.alt));
    }
    SchemaValue::Node(Box::new(node))
}

fn publisher_node(site: &SeoSite) -> Result<SchemaNode, SeoError> {
    let publisher = &site.publisher;
    let (kind, picture) = match publisher.kind {
        PublisherKind::Organization => ("Organization", "logo"),
        PublisherKind::Person => ("Person", "image"),
    };
    let mut node = SchemaNode::new(kind)
        .with_id(publisher_id(site)?)
        .with("name", SchemaValue::text(&publisher.name))
        .with("url", SchemaValue::Url(site_home(site)?));
    if let Some(logo) = &publisher.logo {
        node.set(picture, image_node(logo));
    }
    if !publisher.same_as.is_empty() {
        let profiles = publisher
            .same_as
            .iter()
            .cloned()
            .map(SchemaValue::Url)
            .collect();
        node.set("sameAs", SchemaValue::List(profiles));
    }
    if let Some(email) = &publisher.email {
        node.set("email", SchemaValue::text(email));
    }
    if let Some(telephone) = &publisher.telephone {
        node.set("telephone", SchemaValue::text(telephone));
    }
    if let (PublisherKind::Organization, Some(date)) = (publisher.kind, publisher.founding_date) {
        node.set("foundingDate", SchemaValue::Date(date));
    }
    Ok(node)
}

fn website_node(site: &SeoSite) -> Result<SchemaNode, SeoError> {
    let home = site_home(site)?;
    let mut node = SchemaNode::new("WebSite")
        .with_id(node_id(&home, "website")?)
        .with("name", SchemaValue::text(&site.name))
        .with("url", SchemaValue::Url(home))
        .with(
            "inLanguage",
            SchemaValue::text(site.default_language.as_str()),
        )
        .with("publisher", SchemaValue::Reference(publisher_id(site)?));
    if !site.tagline.is_empty() {
        node.set("description", SchemaValue::text(&site.tagline));
    }
    if let SiteSearch::On(path) = &site.search {
        let results = site.url_for(&site.default_language, path)?;
        let entry = SchemaNode::new("EntryPoint").with(
            "urlTemplate",
            SchemaValue::Text(format!("{}?q={{search_term_string}}", results.as_str())),
        );
        let action = SchemaNode::new("SearchAction")
            .with("target", SchemaValue::Node(Box::new(entry)))
            .with(
                "query-input",
                SchemaValue::text("required name=search_term_string"),
            );
        node.set("potentialAction", SchemaValue::Node(Box::new(action)));
    }
    Ok(node)
}

/// The nodes every page carries: the publisher and the `WebSite`.
///
/// # Errors
///
/// [`SeoError::Url`] when an address cannot be built.
pub fn site_graph(site: &SeoSite) -> Result<SchemaGraph, SeoError> {
    Ok(SchemaGraph::new(vec![
        publisher_node(site)?,
        website_node(site)?,
    ]))
}
