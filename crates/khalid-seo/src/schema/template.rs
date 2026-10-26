//! Schema templates: one schema.org node for every post in a category or
//! collection, such as a `Course` for every post in "On-page SEO".

use std::fmt;

use crate::schema::node_id;
use crate::schema::page_graph::page_id;
use crate::{MetaTemplate, PageKind, SchemaNode, SchemaValue, SeoError, SeoPage, SeoSite};

/// A schema.org type name, such as `Course`: one word of ASCII letters and
/// digits that starts with a capital letter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaType(String);

impl SchemaType {
    /// Checks that `name` has the shape of a schema.org type name.
    ///
    /// # Errors
    ///
    /// [`SeoError::SchemaType`] when `name` is empty, starts with anything
    /// but a capital letter, or holds a character other than an ASCII letter
    /// or digit.
    pub fn try_new(name: &str) -> Result<Self, SeoError> {
        if is_word(name, char::is_ascii_uppercase) {
            Ok(Self(name.to_owned()))
        } else {
            Err(SeoError::SchemaType {
                name: name.to_owned(),
            })
        }
    }

    /// The type name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SchemaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether `text` is one ASCII word whose first character passes `first`.
fn is_word(text: &str, first: fn(&char) -> bool) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| first(&c)) && chars.all(|c| c.is_ascii_alphanumeric())
}

/// Which posts a schema template applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplateScope {
    /// Every post and page whose category has exactly this name.
    Category(String),
    /// Every post and page in the collection with exactly this name, such as `guides`.
    Collection(String),
}

impl TemplateScope {
    fn covers(&self, page: &SeoPage) -> bool {
        let (wanted, actual) = match self {
            Self::Category(name) => (name, &page.category),
            Self::Collection(name) => (name, &page.collection),
        };
        matches!(page.kind, PageKind::Post | PageKind::Page)
            && actual.as_deref() == Some(wanted.as_str())
    }
}

/// One schema.org node written for every post in a scope, its properties
/// filled from the post with the same variables as a title template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaTemplate {
    scope: TemplateScope,
    kind: SchemaType,
    properties: Vec<(String, MetaTemplate)>,
}

impl SchemaTemplate {
    /// A template that writes a node of `kind` for every post in `scope`,
    /// with no properties yet.
    pub fn new(scope: TemplateScope, kind: SchemaType) -> Self {
        Self {
            scope,
            kind,
            properties: Vec::new(),
        }
    }

    /// Sets the property `name` to `value`. A property set before keeps its
    /// place and takes the new template.
    ///
    /// # Errors
    ///
    /// [`SeoError::SchemaProperty`] when `name` is not one word of ASCII
    /// letters and digits that starts with a small letter, such as `provider`.
    pub fn set(&mut self, name: &str, value: MetaTemplate) -> Result<(), SeoError> {
        if !is_word(name, char::is_ascii_lowercase) {
            return Err(SeoError::SchemaProperty {
                name: name.to_owned(),
            });
        }
        match self
            .properties
            .iter_mut()
            .find(|(existing, _)| existing == name)
        {
            Some((_, slot)) => *slot = value,
            None => self.properties.push((name.to_owned(), value)),
        }
        Ok(())
    }

    /// Which posts the template applies to.
    pub fn scope(&self) -> &TemplateScope {
        &self.scope
    }

    /// The schema.org type of the node it writes.
    pub fn kind(&self) -> &SchemaType {
        &self.kind
    }

    /// The node for `page`: its `@id` is `<page>#template-` and the type in
    /// small letters, so it never meets the id of a node the page writes
    /// itself, such as `#webpage`. Then its `mainEntityOfPage`, the page
    /// node, and each property whose template does not render empty.
    fn node(&self, site: &SeoSite, page: &SeoPage) -> Result<SchemaNode, SeoError> {
        let url = site.url_for(&page.language, &page.path)?;
        let fragment = format!("template-{}", self.kind.as_str().to_ascii_lowercase());
        let mut node = SchemaNode::new(self.kind.as_str())
            .with_id(node_id(&url, &fragment)?)
            .with(
                "mainEntityOfPage",
                SchemaValue::Reference(page_id(site, page)?),
            );
        for (name, template) in &self.properties {
            let text = template.render(site, page);
            if !text.is_empty() {
                node.set(name, SchemaValue::text(&text));
            }
        }
        Ok(node)
    }
}

/// The nodes of the site's schema templates that cover `page`, in the order
/// the site lists them. When two templates of one type cover the page, the
/// first wins, so no two nodes share an `@id`.
pub(crate) fn template_nodes(site: &SeoSite, page: &SeoPage) -> Result<Vec<SchemaNode>, SeoError> {
    let mut nodes: Vec<SchemaNode> = Vec::new();
    for template in site
        .schema_templates
        .iter()
        .filter(|t| t.scope.covers(page))
    {
        if nodes
            .iter()
            .all(|node| node.kind() != template.kind.as_str())
        {
            nodes.push(template.node(site, page)?);
        }
    }
    Ok(nodes)
}
