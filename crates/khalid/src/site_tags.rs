//! The Admin API's tags on a running site: read from the `tags` of every
//! post file, published in `content/` and drafts in `drafts/`, and
//! renamed or removed by rewriting that one line in each (RULES.md
//! BL-RUN-9). A post last changed when its file did.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::entry_store::{EntryError, EntryStoreError};
use khalid_api::resources::{AdminTag, TagEdit};
use khalid_api::tag_store::{TagStore, TaggedPost, current, retagged, tag_id, tags_of, unnamed};
use khalid_content::{ContentTree, PostFront};
use khalid_core::{Config, Locale, SlugMode};

use crate::atomic;
use crate::entry_files::front;
use crate::post_text::{PostText, quoted};
use crate::settings_text::put;

/// The collections that hold something other than posts.
const NOT_POSTS: [&str; 6] = [
    "pages",
    "authors",
    "glossary",
    "case-studies",
    "changelog",
    "snippets",
];

/// The tags of the posts in one site folder. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct SiteTags {
    site: PathBuf,
    slugs: SlugMode,
    /// Held while a change is checked and written, so no two interleave.
    writing: Arc<Mutex<()>>,
}

/// A post file and the tags it holds.
struct PostFile {
    path: PathBuf,
    text: PostText,
    tagged: TaggedPost,
}

impl SiteTags {
    /// The tags of the posts in `site`, made into slugs as `config` says.
    /// Every step runs on a blocking thread (RULES.md BL-RUN-7).
    pub(crate) fn new(site: &Path, config: &Config) -> Self {
        Self {
            site: site.to_path_buf(),
            slugs: config.slugs(),
            writing: Arc::new(Mutex::new(())),
        }
    }

    /// Every post file, published and draft.
    fn posts(&self) -> Result<Vec<PostFile>, EntryStoreError> {
        let tree = ContentTree::walk(&self.site).map_err(EntryStoreError::new)?;
        let mut posts = Vec::new();
        for language in tree.languages() {
            for collection in language.collections() {
                if NOT_POSTS.contains(&collection.name()) {
                    continue;
                }
                for entry in collection.entries() {
                    let mut path = self.site.clone();
                    path.push(entry.path());
                    posts.push(post_file(path, language.locale())?);
                }
            }
        }
        posts.extend(self.drafts()?);
        Ok(posts)
    }

    /// The drafts in `drafts/<language>/`.
    fn drafts(&self) -> Result<Vec<PostFile>, EntryStoreError> {
        let mut drafts = Vec::new();
        let mut folder = self.site.clone();
        folder.push("drafts");
        for language in listed(&folder)? {
            let Some(locale) = language.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Ok(locale) = Locale::try_new(locale) else {
                continue;
            };
            for path in listed(&language)? {
                if path.extension().is_some_and(|extension| extension == "md") {
                    drafts.push(post_file(path, &locale)?);
                }
            }
        }
        Ok(drafts)
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.writing.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Renames tag `id` to `name`, or removes it when `name` is `None`, in
    /// every post that has it, and returns the tag it became, if any.
    fn retag(
        &self,
        id: &str,
        name: Option<&str>,
        seen: DateTime<Utc>,
    ) -> Result<Option<AdminTag>, EntryError> {
        let _writing = self.lock();
        let posts = self.posts()?;
        let tagged: Vec<_> = posts.iter().map(|post| post.tagged.clone()).collect();
        let tag = current(&tagged, id, seen, self.slugs)?;
        let new_id = match name {
            Some(name) => Some(tag_id(tag.language(), name, self.slugs).ok_or_else(unnamed)?),
            None => None,
        };
        for mut post in posts {
            if let Some(tags) = retagged(&post.tagged, id, name, self.slugs) {
                let names: Vec<_> = tags.iter().map(|tag| quoted(tag)).collect();
                let line = (!names.is_empty()).then(|| format!("[{}]", names.join(", ")));
                put(&mut post.text, "tags", line);
                atomic::replace(&post.path, &post.text.to_text()).map_err(EntryStoreError::new)?;
            }
        }
        let tagged: Vec<_> = self.posts()?.into_iter().map(|post| post.tagged).collect();
        let tags = tags_of(&tagged, self.slugs);
        Ok(tags
            .into_iter()
            .find(|tag| Some(tag.id()) == new_id.as_deref()))
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
        let tags = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || step(&tags))
                .await
                .map_err(|error| E::from(EntryStoreError::new(error)))?
        })
    }
}

impl TagStore for SiteTags {
    fn list(&self) -> BoxFuture<'_, Result<Vec<AdminTag>, EntryStoreError>> {
        self.blocking(|tags| {
            let posts = tags.posts()?;
            let tagged: Vec<_> = posts.into_iter().map(|post| post.tagged).collect();
            Ok(tags_of(&tagged, tags.slugs))
        })
    }

    fn rename<'a>(
        &'a self,
        id: &'a str,
        edit: TagEdit,
    ) -> BoxFuture<'a, Result<AdminTag, EntryError>> {
        let id = id.to_owned();
        self.blocking(move |tags| {
            let renamed = tags.retag(&id, Some(edit.name()), edit.updated_at())?;
            renamed.ok_or(EntryError::NotFound)
        })
    }

    fn remove<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), EntryError>> {
        let id = id.to_owned();
        self.blocking(move |tags| tags.retag(&id, None, updated_at).map(|_| ()))
    }
}

/// The post file at `path`, in `language`.
fn post_file(path: PathBuf, language: &Locale) -> Result<PostFile, EntryStoreError> {
    let text = fs::read_to_string(&path).map_err(EntryStoreError::new)?;
    let why = || io::Error::other(format!("{} has no front matter", path.display()));
    let text = PostText::parse(&text).ok_or_else(|| EntryStoreError::new(why()))?;
    let post: PostFront = front(&text)?;
    let changed = fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .map_err(EntryStoreError::new)?;
    let tagged = TaggedPost {
        language: language.clone(),
        tags: post.tags,
        updated_at: changed.into(),
    };
    Ok(PostFile { path, text, tagged })
}

/// The items in `folder`, sorted; none when it does not exist.
fn listed(folder: &Path) -> Result<Vec<PathBuf>, EntryStoreError> {
    let items = match fs::read_dir(folder) {
        Ok(items) => items,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(EntryStoreError::new(error)),
    };
    let mut paths = Vec::new();
    for item in items {
        paths.push(item.map_err(EntryStoreError::new)?.path());
    }
    paths.sort();
    Ok(paths)
}
