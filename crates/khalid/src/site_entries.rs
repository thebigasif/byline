//! The Admin API's pages, snippets and authors as files in
//! `content/<language>/<folder>/`, written atomically (RULES.md BL-RUN-9).
//! Each last changed when its file did, so a hand edit makes a copy stale.

use std::fs;
use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::entry_store::{Entry, EntryError, EntryStore, EntryStoreError};
use khalid_core::{Config, Locale, Slug};

use crate::atomic;
use crate::post_text::PostText;

/// How one kind of entry lives in a file.
pub(crate) trait EntryFile: Entry {
    /// The folder under each language, such as `pages`.
    const FOLDER: &'static str;

    /// The entry the file `text` holds, last changed at `changed`, or an
    /// error when its front matter cannot be read.
    fn read(
        key: (Locale, Slug),
        text: &PostText,
        changed: DateTime<Utc>,
    ) -> Result<Self, EntryStoreError>;

    /// The slug in the file's front matter, if any, or an error when its
    /// front matter cannot be read.
    fn slug_in(text: &PostText) -> Result<Option<Slug>, EntryStoreError>;

    /// The slug a new entry asks for.
    fn slug(new: &Self::New) -> &Slug;

    /// The file of a new entry.
    fn text(new: &Self::New) -> PostText;

    /// Writes `edit` into the file, leaving every other line as it was.
    fn edit(text: &mut PostText, edit: &Self::Edit);
}

/// The entries of one kind in a site folder. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct SiteEntries<T> {
    site: Arc<Site>,
    kind: PhantomData<fn() -> T>,
}

#[derive(Debug)]
struct Site {
    folder: PathBuf,
    languages: Vec<Locale>,
    default_language: Locale,
    /// Held while a change is checked and written, so two never interleave.
    writing: Mutex<()>,
}

/// An entry's file, found by its identity.
struct Found<T> {
    path: PathBuf,
    entry: T,
}

impl<T: EntryFile> SiteEntries<T> {
    /// The entries in the site folder `folder`, in the languages `config`
    /// names. Every step runs on a blocking thread (RULES.md BL-RUN-7).
    pub(crate) fn new(folder: &Path, config: &Config) -> Self {
        let site = Site {
            folder: folder.to_path_buf(),
            languages: config.languages().to_vec(),
            default_language: config.default_language().clone(),
            writing: Mutex::new(()),
        };
        Self {
            site: Arc::new(site),
            kind: PhantomData,
        }
    }

    /// `content/<language>/<folder>/`.
    fn folder(&self, language: &Locale) -> PathBuf {
        let mut folder = self.site.folder.clone();
        folder.extend(["content", language.as_str(), T::FOLDER]);
        folder
    }

    /// Every entry in `language`: a markdown file, or a folder with an
    /// `index.md`. A missing folder holds none.
    fn read_language(&self, language: &Locale) -> Result<Vec<Found<T>>, EntryStoreError> {
        let listed = match fs::read_dir(self.folder(language)) {
            Ok(listed) => listed,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(EntryStoreError::new(error)),
        };
        let mut found = Vec::new();
        for item in listed {
            let path = item.map_err(EntryStoreError::new)?.path();
            if let Some((path, stem)) = entry_file(path) {
                found.push(Self::read_file(language, path, &stem)?);
            }
        }
        found.sort_by(|a, b| a.entry.id().cmp(b.entry.id()));
        Ok(found)
    }

    fn read_file(
        language: &Locale,
        path: PathBuf,
        stem: &str,
    ) -> Result<Found<T>, EntryStoreError> {
        let text = fs::read_to_string(&path).map_err(EntryStoreError::new)?;
        let text = PostText::parse(&text).ok_or_else(|| unreadable(&path))?;
        let changed = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .map_err(EntryStoreError::new)?;
        let from_stem = || Slug::try_new_unicode(stem).map_err(|_| unreadable(&path));
        let slug = T::slug_in(&text)?.map_or_else(from_stem, Ok)?;
        let entry = T::read((language.clone(), slug), &text, changed.into())?;
        Ok(Found { path, entry })
    }

    fn list_now(&self) -> Result<Vec<T>, EntryStoreError> {
        let mut entries = Vec::new();
        for language in &self.site.languages {
            entries.extend(self.read_language(language)?.into_iter().map(|f| f.entry));
        }
        Ok(entries)
    }

    /// The entry with identity `<language>.<slug>`, or `None`.
    fn find(&self, id: &str) -> Result<Option<Found<T>>, EntryStoreError> {
        let named = id.split_once('.').map(|(language, _)| language);
        let languages = &self.site.languages;
        let Some(language) = languages.iter().find(|l| Some(l.as_str()) == named) else {
            return Ok(None);
        };
        let found = self.read_language(language)?;
        Ok(found.into_iter().find(|found| found.entry.id() == id))
    }

    fn create_now(&self, new: &T::New) -> Result<T, EntryError> {
        let language = T::language(new).unwrap_or(&self.site.default_language);
        if !self.site.languages.contains(language) {
            return Err(EntryError::Unsupported {
                field: "language",
                why: "The site is not written in that language, so the entry would never be shown.",
            });
        }
        let id = format!("{language}.{}", T::slug(new));
        let _writing = self.lock();
        if self.find(&id)?.is_some() {
            return Err(EntryError::Taken(id));
        }
        let folder = self.folder(language);
        fs::create_dir_all(&folder).map_err(EntryStoreError::new)?;
        let name = format!("{}.md", T::slug(new));
        match atomic::create(&folder, &name, &T::text(new).to_text()) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(EntryError::Taken(id));
            }
            written => written.map_err(EntryStoreError::new)?,
        }
        self.written(&id)
    }

    /// Holds the write lock, so no two changes interleave.
    fn lock(&self) -> MutexGuard<'_, ()> {
        self.site
            .writing
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The entry `id` as just written.
    fn written(&self, id: &str) -> Result<T, EntryError> {
        let found = self.find(id)?.ok_or(EntryError::NotFound)?;
        Ok(found.entry)
    }

    /// The entry `id`, when it has not changed since `seen`.
    fn current(&self, id: &str, seen: DateTime<Utc>) -> Result<Found<T>, EntryError> {
        let found = self.find(id)?.ok_or(EntryError::NotFound)?;
        let updated_at = found.entry.updated_at();
        if updated_at == seen {
            Ok(found)
        } else {
            Err(EntryError::Stale { updated_at })
        }
    }

    fn update_now(&self, id: &str, edit: &T::Edit) -> Result<T, EntryError> {
        let _writing = self.lock();
        let found = self.current(id, T::seen(edit))?;
        let text = fs::read_to_string(&found.path).map_err(EntryStoreError::new)?;
        let mut text = PostText::parse(&text).ok_or_else(|| unreadable(&found.path))?;
        T::edit(&mut text, edit);
        atomic::replace(&found.path, &text.to_text()).map_err(EntryStoreError::new)?;
        self.written(id)
    }

    fn delete_now(&self, id: &str, seen: DateTime<Utc>) -> Result<(), EntryError> {
        let _writing = self.lock();
        let found = self.current(id, seen)?;
        let removed = match found.path.parent() {
            Some(folder) if found.path.ends_with("index.md") => fs::remove_dir_all(folder),
            _ => fs::remove_file(&found.path),
        };
        removed.map_err(EntryStoreError::new)?;
        Ok(())
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
        let entries = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || step(&entries))
                .await
                .map_err(|error| E::from(EntryStoreError::new(error)))?
        })
    }
}

impl<T: EntryFile> EntryStore<T> for SiteEntries<T> {
    fn list(&self) -> BoxFuture<'_, Result<Vec<T>, EntryStoreError>> {
        self.blocking(Self::list_now)
    }

    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<T>, EntryStoreError>> {
        let id = id.to_owned();
        self.blocking(move |entries| Ok(entries.find(&id)?.map(|found| found.entry)))
    }

    fn create(&self, new: T::New) -> BoxFuture<'_, Result<T, EntryError>> {
        self.blocking(move |entries| entries.create_now(&new))
    }

    fn update<'a>(&'a self, id: &'a str, edit: T::Edit) -> BoxFuture<'a, Result<T, EntryError>> {
        let id = id.to_owned();
        self.blocking(move |entries| entries.update_now(&id, &edit))
    }

    fn delete<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), EntryError>> {
        let id = id.to_owned();
        self.blocking(move |entries| entries.delete_now(&id, updated_at))
    }
}

/// The markdown file of an entry folder item, `about.md` or `about/index.md`,
/// and its fallback slug; `None` for hidden files and `_index.md`.
fn entry_file(path: PathBuf) -> Option<(PathBuf, String)> {
    let name = path.file_name()?.to_str()?.to_owned();
    if name.starts_with(['.', '_']) {
        return None;
    }
    if path.is_dir() {
        let mut index = path;
        index.push("index.md");
        return index.is_file().then_some((index, name));
    }
    let stem = name.strip_suffix(".md")?.to_owned();
    Some((path, stem))
}

/// The error for a file that cannot be read as an entry.
fn unreadable(path: &Path) -> EntryStoreError {
    let why = format!("{} has no front matter Khalid can read", path.display());
    EntryStoreError::new(io::Error::other(why))
}

#[cfg(test)]
#[path = "site_entries_tests.rs"]
mod tests;
