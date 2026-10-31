//! The site's schema templates as the Admin API reads and replaces them:
//! `[[seo.schema]]` in `site.toml`, edited in place so every other line,
//! comment and blank keeps its place (G54, KH-SEO-4). The running server
//! read `site.toml` at start, so a change applies after a restart.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::{fs, io};

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::resources::NonBlank;
use khalid_api::schema_template_store::{
    AdminSchemaTemplate, SchemaTemplateStore, SchemaTemplates, TemplateStoreError, TemplateTarget,
    TemplatesChange,
};
use khalid_content::{Collection, ContentTree, Language};
use khalid_core::{Config, Environment, SchemaTemplateSettings};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use crate::{atomic, site_config};

/// Content folders that hold something other than posts, which schema
/// templates never cover.
const NOT_POSTS: [&str; 6] = [
    "pages",
    "authors",
    "glossary",
    "case-studies",
    "changelog",
    "snippets",
];

/// The schema templates of the site in one folder. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct SiteSchemaTemplates {
    site: PathBuf,
    /// The templates the running server started with.
    running: Vec<SchemaTemplateSettings>,
    /// Held while a change is written, so two never interleave.
    writing: Arc<Mutex<()>>,
}

impl SiteSchemaTemplates {
    /// The templates in `site`'s `site.toml`, for a server that started
    /// with `running`.
    pub(crate) fn new(site: &Path, running: &Config) -> Self {
        Self {
            site: site.to_path_buf(),
            running: running.seo().schema.clone(),
            writing: Arc::new(Mutex::new(())),
        }
    }

    fn path(&self) -> PathBuf {
        let mut path = self.site.clone();
        path.push(Config::FILE_NAME);
        path
    }

    /// The file's text and when it last changed.
    fn text(&self) -> Result<(String, DateTime<Utc>), TemplateStoreError> {
        let path = self.path();
        let text = fs::read_to_string(&path).map_err(store)?;
        let changed = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .map_err(store)?;
        Ok((text, changed.into()))
    }

    /// `text` as a config, when it passes every check made at start.
    fn checked(&self, text: &str) -> Result<Config, TemplateStoreError> {
        let config = Config::from_toml(&self.path(), text, &Environment::empty()).map_err(store)?;
        // The API checks each template first, so only a mistake elsewhere
        // in the file, made by hand, ends here.
        site_config::check(&self.site, &config)
            .map_err(|message| store(io::Error::other(message.to_string())))?;
        Ok(config)
    }

    fn read_now(&self) -> Result<SchemaTemplates, TemplateStoreError> {
        let (text, changed) = self.text()?;
        let config = self.checked(&text)?;
        let held = &config.seo().schema;
        Ok(SchemaTemplates {
            templates: held.iter().filter_map(admin_template).collect(),
            collections: self.collections(),
            updated_at: changed,
            restart_needed: *held != self.running,
        })
    }

    fn replace_now(&self, change: &TemplatesChange) -> Result<SchemaTemplates, TemplateStoreError> {
        let _writing = self.lock();
        let (text, changed) = self.text()?;
        if changed != change.updated_at {
            return Err(TemplateStoreError::Stale {
                updated_at: changed,
            });
        }
        let mut document = text.parse::<DocumentMut>().map_err(store)?;
        set_templates(&mut document, &change.templates);
        let text = document.to_string();
        self.checked(&text)?;
        atomic::replace(&self.path(), &text).map_err(store)?;
        self.read_now()
    }

    /// The collections that hold posts, in every language: `posts`, and
    /// any other folder of posts under `content/`.
    fn collections(&self) -> Vec<String> {
        let mut names = BTreeSet::from(["posts".to_owned()]);
        if let Ok(tree) = ContentTree::walk(&self.site) {
            let folders = tree.languages().iter().flat_map(Language::collections);
            let posts = folders
                .map(Collection::name)
                .filter(|name| !NOT_POSTS.contains(name));
            names.extend(posts.map(str::to_owned));
        }
        names.into_iter().collect()
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.writing.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Runs `step` on a blocking thread, since it reads or writes files
    /// (RULES.md BL-RUN-7).
    fn blocking<R: Send + 'static>(
        &self,
        step: impl FnOnce(&Self) -> Result<R, TemplateStoreError> + Send + 'static,
    ) -> BoxFuture<'static, Result<R, TemplateStoreError>> {
        let templates = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || step(&templates))
                .await
                .map_err(store)?
        })
    }
}

impl SchemaTemplateStore for SiteSchemaTemplates {
    fn read(&self) -> BoxFuture<'_, Result<SchemaTemplates, TemplateStoreError>> {
        self.blocking(Self::read_now)
    }

    fn replace(
        &self,
        change: TemplatesChange,
    ) -> BoxFuture<'_, Result<SchemaTemplates, TemplateStoreError>> {
        self.blocking(move |templates| templates.replace_now(&change))
    }
}

/// One `[[seo.schema]]` entry in the Admin API's shape, or `None` for one
/// whose name is blank, which the start check refuses.
fn admin_template(held: &SchemaTemplateSettings) -> Option<AdminSchemaTemplate> {
    let (applies_to, name) = match (&held.collection, &held.category) {
        (Some(collection), _) => (TemplateTarget::Collection, collection),
        (None, Some(category)) => (TemplateTarget::Category, category),
        (None, None) => return None,
    };
    let mut template = AdminSchemaTemplate::new(applies_to, NonBlank::try_new(name)?, &held.kind);
    template.properties.clone_from(&held.properties);
    Some(template)
}

/// Puts `templates` in place of every `[[seo.schema]]` in `document`, or
/// removes the key when there are none. The `[seo]` table is made without
/// a header of its own when the file had none.
fn set_templates(document: &mut DocumentMut, templates: &[AdminSchemaTemplate]) {
    let seo = document.entry("seo").or_insert_with(|| {
        let mut table = Table::new();
        table.set_implicit(true);
        Item::Table(table)
    });
    let Some(seo) = seo.as_table_like_mut() else {
        return;
    };
    if templates.is_empty() {
        seo.remove("schema");
        return;
    }
    let mut entries = ArrayOfTables::new();
    for template in templates {
        let mut entry = Table::new();
        let key = match template.applies_to {
            TemplateTarget::Category => "category",
            _ => "collection",
        };
        entry.insert(key, value(template.name.as_str()));
        entry.insert("type", value(template.kind.as_str()));
        if !template.properties.is_empty() {
            let mut properties = Table::new();
            for (name, text) in &template.properties {
                properties.insert(name, value(text.as_str()));
            }
            entry.insert("properties", Item::Table(properties));
        }
        entries.push(entry);
    }
    seo.insert("schema", Item::ArrayOfTables(entries));
}

fn store(error: impl std::error::Error + Send + Sync + 'static) -> TemplateStoreError {
    TemplateStoreError::Store(Box::new(error))
}

#[cfg(test)]
#[path = "site_schema_templates_tests.rs"]
mod tests;
