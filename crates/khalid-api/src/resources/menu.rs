//! A menu: the links of one navigation area in one language.

use khalid_core::{Locale, Slug};
use serde::Serialize;
use utoipa::ToSchema;

use super::{Listable, display};
use crate::query::{FieldValues, Literal};

/// One menu in one language, in API shape, such as the header menu
/// `primary` or the `footer` menu. An empty menu shows nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Menu {
    /// The language and the menu, such as `en.primary`.
    id: String,
    /// Which menu, such as `primary` or `footer`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "primary")]
    slug: Slug,
    /// The language, such as `en` or `de`.
    #[serde(serialize_with = "display")]
    #[schema(value_type = String, example = "en")]
    language: Locale,
    /// The links, in the order readers see them.
    links: Vec<MenuLink>,
}

/// One link in a menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct MenuLink {
    /// The words readers see.
    label: String,
    /// Where it goes: a path on the site, such as `/blog/about/`, or a full
    /// address.
    #[schema(example = "/blog/about/")]
    url: String,
}

impl Menu {
    /// The menu `slug` in `language`, with no links. The identity is
    /// `language.slug`.
    pub fn new(slug: Slug, language: Locale) -> Self {
        Self {
            id: format!("{}.{}", language.as_str(), slug.as_str()),
            slug,
            language,
            links: Vec::new(),
        }
    }

    /// The same menu with these links, in order.
    #[must_use]
    pub fn with_links(self, links: Vec<MenuLink>) -> Self {
        Self { links, ..self }
    }
}

impl MenuLink {
    /// A link showing `label` that goes to `url`.
    pub fn new(label: &str, url: &str) -> Self {
        Self {
            label: label.to_owned(),
            url: url.to_owned(),
        }
    }
}

impl Listable for Menu {
    const FIELDS: &'static [&'static str] = &["id", "slug", "language"];

    fn id(&self) -> &str {
        &self.id
    }

    fn slug(&self) -> &Slug {
        &self.slug
    }
}

impl FieldValues for Menu {
    fn values(&self, field: &str) -> Vec<Literal> {
        let text = |value: &str| vec![Literal::Text(value.to_owned())];
        match field {
            "id" => text(&self.id),
            "slug" => text(self.slug.as_str()),
            "language" => text(self.language.as_str()),
            _ => Vec::new(),
        }
    }
}
