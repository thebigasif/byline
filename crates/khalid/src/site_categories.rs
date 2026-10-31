//! The Admin API's categories on a running site: the entries of
//! `categories.toml`, edited in place so its comments and layout stay as
//! written, and written atomically (RULES.md BL-RUN-9). Every category last
//! changed when the file did, so a hand edit makes an older copy stale.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::entry_store::{Entry, EntryError, EntryStore, EntryStoreError};
use khalid_api::resources::{AdminCategory, CategoryEdit, NewCategory};
use khalid_core::{Config, Locale, Slug};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use crate::atomic;
use crate::site_tags::post_files;

/// The file's name in the site folder.
const FILE: &str = "categories.toml";

/// The categories of one site folder. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct SiteCategories {
    site: PathBuf,
    languages: Vec<Locale>,
    /// Held while a change is checked and written, so no two interleave.
    writing: Arc<Mutex<()>>,
}

/// The file as read: its text and when it last changed, or neither.
struct Read {
    document: DocumentMut,
    changed: DateTime<Utc>,
}

impl SiteCategories {
    /// The categories in `site`'s `categories.toml`, named in the
    /// languages `config` lists. Every step runs on a blocking thread
    /// (RULES.md BL-RUN-7).
    pub(crate) fn new(site: &Path, config: &Config) -> Self {
        Self {
            site: site.to_path_buf(),
            languages: config.languages().to_vec(),
            writing: Arc::new(Mutex::new(())),
        }
    }

    fn path(&self) -> PathBuf {
        let mut path = self.site.clone();
        path.push(FILE);
        path
    }

    /// The file, or an empty one dated 1970 when the site has none yet.
    fn read(&self) -> Result<Read, EntryStoreError> {
        let path = self.path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let document = DocumentMut::new();
                return Ok(Read {
                    document,
                    changed: DateTime::UNIX_EPOCH,
                });
            }
            Err(error) => return Err(EntryStoreError::new(error)),
        };
        let document = text.parse::<DocumentMut>().map_err(EntryStoreError::new)?;
        let changed = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .map_err(EntryStoreError::new)?;
        Ok(Read {
            document,
            changed: changed.into(),
        })
    }

    /// Every category in the file's order, each with its post count.
    fn list_now(&self) -> Result<Vec<AdminCategory>, EntryStoreError> {
        let read = self.read()?;
        let mut counts: BTreeMap<Slug, usize> = BTreeMap::new();
        for post in post_files(&self.site)? {
            if let Some(category) = post.category {
                *counts.entry(category).or_default() += 1;
            }
        }
        let tables = read
            .document
            .get("categories")
            .and_then(Item::as_array_of_tables);
        let mut categories = Vec::new();
        for table in tables.into_iter().flat_map(ArrayOfTables::iter) {
            let Some(slug) = slug_of(table) else { continue };
            let names = table
                .iter()
                .filter(|(key, _)| *key != "slug")
                .filter_map(|(key, item)| Some((key.to_owned(), item.as_str()?.to_owned())))
                .collect();
            let posts = counts.get(&slug).copied().unwrap_or_default();
            categories.push(AdminCategory::new(slug, names, posts, read.changed));
        }
        Ok(categories)
    }

    fn get_now(&self, id: &str) -> Result<Option<AdminCategory>, EntryStoreError> {
        let categories = self.list_now()?;
        Ok(categories
            .into_iter()
            .find(|category| category.slug().as_str() == id))
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.writing.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Refuses names in a language the site is not written in.
    fn check_languages(&self, names: &BTreeMap<String, String>) -> Result<(), EntryError> {
        let known = |code: &String| self.languages.iter().any(|l| l.as_str() == code);
        if names.keys().all(known) {
            Ok(())
        } else {
            Err(EntryError::Unsupported {
                field: "names",
                why: "Each name must be in one of the site's languages, as site.toml lists them.",
            })
        }
    }

    fn create_now(&self, new: &NewCategory) -> Result<AdminCategory, EntryError> {
        self.check_languages(new.names())?;
        let _writing = self.lock();
        let slug = new.slug().as_str();
        if self.get_now(slug)?.is_some() {
            return Err(EntryError::Taken(slug.to_owned()));
        }
        let mut read = self.read()?;
        let entry = read.document.entry("categories");
        let tables = entry.or_insert(Item::ArrayOfTables(ArrayOfTables::new()));
        let tables = tables.as_array_of_tables_mut().ok_or_else(not_a_list)?;
        let mut table = Table::new();
        table.insert("slug", value(slug));
        put_names(&mut table, new.names());
        tables.push(table);
        self.write(&read.document)?;
        self.get_now(slug)?.ok_or(EntryError::NotFound)
    }

    /// The file, when the category `id` is in it and the file has not
    /// changed since `seen`, and the category's place in its list.
    fn current(&self, id: &str, seen: DateTime<Utc>) -> Result<(Read, usize), EntryError> {
        let read = self.read()?;
        let tables = read
            .document
            .get("categories")
            .and_then(Item::as_array_of_tables);
        let at = tables
            .into_iter()
            .flat_map(ArrayOfTables::iter)
            .position(|table| slug_of(table).is_some_and(|slug| slug.as_str() == id))
            .ok_or(EntryError::NotFound)?;
        if read.changed != seen {
            return Err(EntryError::Stale {
                updated_at: read.changed,
            });
        }
        Ok((read, at))
    }

    fn update_now(&self, id: &str, edit: &CategoryEdit) -> Result<AdminCategory, EntryError> {
        self.check_languages(edit.names())?;
        let _writing = self.lock();
        let (mut read, at) = self.current(id, AdminCategory::seen(edit))?;
        let table = read
            .document
            .get_mut("categories")
            .and_then(Item::as_array_of_tables_mut)
            .and_then(|tables| tables.get_mut(at))
            .ok_or(EntryError::NotFound)?;
        table.retain(|key, _| key == "slug");
        put_names(table, edit.names());
        self.write(&read.document)?;
        self.get_now(id)?.ok_or(EntryError::NotFound)
    }

    fn delete_now(&self, id: &str, seen: DateTime<Utc>) -> Result<(), EntryError> {
        let _writing = self.lock();
        let (mut read, at) = self.current(id, seen)?;
        let posts = self.get_now(id)?.map_or(0, |category| category.posts());
        if posts > 0 {
            return Err(EntryError::InUse { posts });
        }
        let tables = read.document.get_mut("categories");
        let tables = tables
            .and_then(Item::as_array_of_tables_mut)
            .ok_or_else(not_a_list)?;
        tables.remove(at);
        self.write(&read.document)
    }

    fn write(&self, document: &DocumentMut) -> Result<(), EntryError> {
        let path = self.path();
        let text = document.to_string();
        let written = if path.exists() {
            atomic::replace(&path, &text)
        } else {
            atomic::create(&self.site, FILE, &text)
        };
        written.map_err(|error| EntryError::Store(EntryStoreError::new(error)))
    }

    /// Runs `step` on a blocking thread, since it reads or writes files.
    fn blocking<R, E>(
        &self,
        step: impl FnOnce(&Self) -> Result<R, E> + Send + 'static,
    ) -> BoxFuture<'static, Result<R, E>>
    where
        R: Send + 'static,
        E: From<EntryStoreError> + Send + 'static,
    {
        let categories = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || step(&categories))
                .await
                .map_err(|error| E::from(EntryStoreError::new(error)))?
        })
    }
}

impl EntryStore<AdminCategory> for SiteCategories {
    fn list(&self) -> BoxFuture<'_, Result<Vec<AdminCategory>, EntryStoreError>> {
        self.blocking(Self::list_now)
    }

    fn get<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<AdminCategory>, EntryStoreError>> {
        let id = id.to_owned();
        self.blocking(move |categories| categories.get_now(&id))
    }

    fn create(&self, new: NewCategory) -> BoxFuture<'_, Result<AdminCategory, EntryError>> {
        self.blocking(move |categories| categories.create_now(&new))
    }

    fn update<'a>(
        &'a self,
        id: &'a str,
        edit: CategoryEdit,
    ) -> BoxFuture<'a, Result<AdminCategory, EntryError>> {
        let id = id.to_owned();
        self.blocking(move |categories| categories.update_now(&id, &edit))
    }

    fn delete<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), EntryError>> {
        let id = id.to_owned();
        self.blocking(move |categories| categories.delete_now(&id, updated_at))
    }
}

/// The slug of one `[[categories]]` table, when it has a valid one.
fn slug_of(table: &Table) -> Option<Slug> {
    let slug = table.get("slug")?.as_str()?;
    Slug::try_new(slug).ok()
}

/// Writes each name under its language code.
fn put_names(table: &mut Table, names: &BTreeMap<String, String>) {
    for (code, name) in names {
        table.insert(code, value(name.as_str()));
    }
}

/// The error for a `categories` key that is not a list of tables.
fn not_a_list() -> EntryError {
    let why = "categories in categories.toml is not a list of [[categories]] tables";
    EntryError::Store(EntryStoreError::new(io::Error::other(why)))
}
