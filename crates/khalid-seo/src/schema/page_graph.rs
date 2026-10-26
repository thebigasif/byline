//! The whole graph of one page: the site nodes, the breadcrumbs and the
//! page node.

use khalid_core::AbsoluteUrl;

use crate::schema::article::article_nodes;
use crate::schema::glossary::{term_node, term_set_node};
use crate::schema::local_business::business_nodes;
use crate::schema::{image_node, node_id, site_home};
use crate::{
    PageKind, SchemaGraph, SchemaNode, SchemaValue, SeoError, SeoPage, SeoSite, breadcrumb_list,
    site_graph,
};

/// The schema.org type of a page of `kind`.
fn page_type(kind: PageKind) -> &'static str {
    match kind {
        PageKind::Tag
        | PageKind::Category
        | PageKind::Collection
        | PageKind::Date
        | PageKind::GlossaryIndex => "CollectionPage",
        PageKind::Author => "ProfilePage",
        PageKind::Search => "SearchResultsPage",
        PageKind::Home
        | PageKind::Post
        | PageKind::Page
        | PageKind::GlossaryTerm
        | PageKind::NotFound => "WebPage",
    }
}

/// The `@id` of the page node, `<page>#webpage`, which an article names as
/// its `mainEntityOfPage`.
pub(crate) fn page_id(site: &SeoSite, page: &SeoPage) -> Result<AbsoluteUrl, SeoError> {
    node_id(&site.url_for(&page.language, &page.path)?, "webpage")
}

fn page_node(site: &SeoSite, page: &SeoPage) -> Result<SchemaNode, SeoError> {
    let url = site.url_for(&page.language, &page.path)?;
    let name = if page.title.is_empty() {
        &site.name
    } else {
        &page.title
    };
    let mut node = SchemaNode::new(page_type(page.kind))
        .with_id(page_id(site, page)?)
        .with("name", SchemaValue::text(name))
        .with("url", SchemaValue::Url(url.clone()))
        .with(
            "isPartOf",
            SchemaValue::Reference(node_id(&site_home(site)?, "website")?),
        )
        .with("inLanguage", SchemaValue::text(page.language.as_str()))
        .with(
            "breadcrumb",
            SchemaValue::Reference(node_id(&url, "breadcrumb")?),
        );
    if !page.excerpt.is_empty() {
        node.set("description", SchemaValue::text(&page.excerpt));
    }
    if let Some(image) = &page.share_image {
        node.set("primaryImageOfPage", image_node(image));
    }
    if let Some(published) = page.published {
        node.set("datePublished", SchemaValue::DateTime(published));
    }
    if let Some(modified) = page.modified {
        node.set("dateModified", SchemaValue::DateTime(modified));
    }
    if !page.speakable.is_empty() {
        let selectors = page
            .speakable
            .iter()
            .map(|s| SchemaValue::text(s))
            .collect();
        let speakable = SchemaNode::new("SpeakableSpecification")
            .with("cssSelector", SchemaValue::List(selectors));
        node.set("speakable", SchemaValue::Node(Box::new(speakable)));
    }
    Ok(node)
}

/// The whole schema graph of `page`: the publisher, the `WebSite`, the
/// `BreadcrumbList`, then the page node, typed by the page's kind:
/// `CollectionPage` for archives, `ProfilePage` for an author,
/// `SearchResultsPage` for search, `WebPage` for the rest. A post adds its
/// article node, typed by its [`ArticleType`](crate::ArticleType), and the
/// `Person` who wrote it. A glossary term adds its `DefinedTerm`, the
/// glossary's A to Z page its `DefinedTermSet`, and the home page one node
/// per location of a local business.
///
/// # Errors
///
/// [`SeoError::Url`] when an address cannot be built.
pub fn page_graph(site: &SeoSite, page: &SeoPage) -> Result<SchemaGraph, SeoError> {
    let mut graph = site_graph(site)?;
    graph.push(breadcrumb_list(site, page)?);
    graph.push(page_node(site, page)?);
    let extra = match page.kind {
        PageKind::Post => article_nodes(site, page, page_id(site, page)?)?,
        PageKind::GlossaryTerm => vec![term_node(site, page)?],
        PageKind::GlossaryIndex => vec![term_set_node(site, page)?],
        PageKind::Home => business_nodes(site)?,
        _ => Vec::new(),
    };
    for node in extra {
        graph.push(node);
    }
    Ok(graph)
}
