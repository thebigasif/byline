//! Glossary schema: each term a `DefinedTerm` in the glossary's
//! `DefinedTermSet`.

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError, SeoPage, SeoSite};

/// The `@id` of the glossary in the page's language: `<glossary>#termset`.
fn term_set_id(site: &SeoSite, page: &SeoPage) -> Result<AbsoluteUrl, SeoError> {
    node_id(&site.url_for(&page.language, &site.glossary)?, "termset")
}

/// The `DefinedTerm` of a glossary term page.
pub(crate) fn term_node(site: &SeoSite, page: &SeoPage) -> Result<SchemaNode, SeoError> {
    let url = site.url_for(&page.language, &page.path)?;
    let mut node = SchemaNode::new("DefinedTerm")
        .with_id(node_id(&url, "term")?)
        .with("name", SchemaValue::text(&page.title))
        .with("url", SchemaValue::Url(url))
        .with(
            "inDefinedTermSet",
            SchemaValue::Reference(term_set_id(site, page)?),
        );
    if !page.excerpt.is_empty() {
        node.set("description", SchemaValue::text(&page.excerpt));
    }
    Ok(node)
}

/// The `DefinedTermSet` of the glossary's A to Z page.
pub(crate) fn term_set_node(site: &SeoSite, page: &SeoPage) -> Result<SchemaNode, SeoError> {
    let mut node = SchemaNode::new("DefinedTermSet")
        .with_id(term_set_id(site, page)?)
        .with("name", SchemaValue::text(&page.title))
        .with(
            "url",
            SchemaValue::Url(site.url_for(&page.language, &page.path)?),
        )
        .with("inLanguage", SchemaValue::text(page.language.as_str()));
    if !page.excerpt.is_empty() {
        node.set("description", SchemaValue::text(&page.excerpt));
    }
    Ok(node)
}
