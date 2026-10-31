//! Where the Admin API reads and replaces the site's schema templates: one
//! schema.org node for every post in a collection, such as a `Course` for
//! every post in `guides` (G54, KH-SEO-4). `khalid serve` keeps them as
//! `[[seo.schema]]` in `site.toml`.

use std::collections::BTreeMap;
use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_seo::{MetaTemplate, SchemaTemplate, SchemaType, TemplateScope};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::content_store::BoxFuture;
use crate::resources::NonBlank;

/// What a schema template's `name` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TemplateTarget {
    /// A collection, such as `guides`.
    Collection,
    /// A category, by its name.
    Category,
}

/// One schema template as the Admin API sends and takes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct AdminSchemaTemplate {
    /// Whether `name` is a collection or a category.
    pub applies_to: TemplateTarget,
    /// The collection or category whose posts get the node.
    #[schema(value_type = String, example = "guides")]
    pub name: NonBlank,
    /// The schema.org type, such as `Course`.
    #[serde(rename = "type")]
    #[schema(example = "Course")]
    pub kind: String,
    /// Each property's value, by property name. A value may use the title
    /// template variables, such as `{title}` and `{site}`.
    #[serde(default)]
    #[schema(example = json!({"provider": "{site}"}))]
    pub properties: BTreeMap<String, String>,
}

impl AdminSchemaTemplate {
    /// A template that writes a node of `kind` for every post `applies_to`
    /// and `name` pick out, with no properties yet.
    pub fn new(applies_to: TemplateTarget, name: NonBlank, kind: &str) -> Self {
        Self {
            applies_to,
            name,
            kind: kind.to_owned(),
            properties: BTreeMap::new(),
        }
    }

    /// The template as `khalid-seo` writes it.
    ///
    /// # Errors
    ///
    /// Says what is wrong when the type is not a schema.org type name, a
    /// property name is not one, or a value has a brace without its
    /// partner or an unknown variable.
    pub fn checked(&self) -> Result<SchemaTemplate, String> {
        let name = self.name.as_str().to_owned();
        let scope = match self.applies_to {
            TemplateTarget::Collection => TemplateScope::Collection(name),
            TemplateTarget::Category => TemplateScope::Category(name),
        };
        let kind = SchemaType::try_new(&self.kind).map_err(|error| error.to_string())?;
        let mut template = SchemaTemplate::new(scope, kind);
        for (property, value) in &self.properties {
            let value = MetaTemplate::parse(value).map_err(|error| error.to_string())?;
            template
                .set(property, value)
                .map_err(|error| error.to_string())?;
        }
        Ok(template)
    }
}

/// The site's schema templates, in the order `site.toml` lists them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct SchemaTemplates {
    /// Every template. When two of one type cover a post, the first wins.
    pub templates: Vec<AdminSchemaTemplate>,
    /// The site's collections, to pick a template's collection from.
    #[schema(example = json!(["posts", "guides"]))]
    pub collections: Vec<String>,
    /// When `site.toml` last changed. A change sends it back.
    pub updated_at: DateTime<Utc>,
    /// Whether the running server still uses older templates, so a restart
    /// is needed before previews and served pages show these.
    pub restart_needed: bool,
}

/// A new list of schema templates, made on the copy `updated_at` names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TemplatesChange {
    /// Every template, in order. An empty list removes them all.
    pub templates: Vec<AdminSchemaTemplate>,
    /// The `updated_at` of the copy the change was made on.
    pub updated_at: DateTime<Utc>,
}

/// The site's schema templates, for the Admin API. `khalid serve` keeps
/// them in `site.toml`; [`MemorySchemaTemplates`], from the `test-util`
/// feature, keeps them in memory for tests.
pub trait SchemaTemplateStore: Send + Sync {
    /// The templates as `site.toml` holds them now.
    ///
    /// # Errors
    ///
    /// Returns [`TemplateStoreError::Store`] when they cannot be read.
    fn read(&self) -> BoxFuture<'_, Result<SchemaTemplates, TemplateStoreError>>;

    /// Puts `change`'s templates in place of all of them, but only when
    /// they have not changed since the copy it was made on (BL-API-7).
    /// The caller has checked each template.
    ///
    /// # Errors
    ///
    /// Returns [`TemplateStoreError::Stale`] or [`TemplateStoreError::Store`].
    fn replace(
        &self,
        change: TemplatesChange,
    ) -> BoxFuture<'_, Result<SchemaTemplates, TemplateStoreError>>;
}

/// Why schema templates were not read or saved.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TemplateStoreError {
    /// The templates changed after the copy the change was made on.
    #[error("the schema templates changed after the copy the change was made on")]
    Stale {
        /// When they last changed.
        updated_at: DateTime<Utc>,
    },
    /// `site.toml` could not be read or written.
    #[error("site.toml could not be read or written")]
    Store(#[source] Box<dyn Error + Send + Sync>),
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemorySchemaTemplates;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::sync::{Mutex, PoisonError};

    use super::{
        BoxFuture, SchemaTemplateStore, SchemaTemplates, TemplateStoreError, TemplatesChange,
    };

    /// Schema templates held in memory, for tests. A change stamps them
    /// one second later and asks for a restart.
    #[derive(Debug)]
    pub struct MemorySchemaTemplates(Mutex<SchemaTemplates>);

    impl MemorySchemaTemplates {
        /// A store holding `templates`.
        pub fn new(templates: SchemaTemplates) -> Self {
            Self(Mutex::new(templates))
        }
    }

    impl SchemaTemplateStore for MemorySchemaTemplates {
        fn read(&self) -> BoxFuture<'_, Result<SchemaTemplates, TemplateStoreError>> {
            let held = self
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            Box::pin(async move { Ok(held) })
        }

        fn replace(
            &self,
            change: TemplatesChange,
        ) -> BoxFuture<'_, Result<SchemaTemplates, TemplateStoreError>> {
            let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let answer = if change.updated_at == held.updated_at {
                held.templates = change.templates;
                held.updated_at += chrono::TimeDelta::seconds(1);
                held.restart_needed = true;
                Ok(held.clone())
            } else {
                let updated_at = held.updated_at;
                Err(TemplateStoreError::Stale { updated_at })
            };
            Box::pin(async move { answer })
        }
    }
}
