//! The trash of a running site (KH-EDT-6): a deleted post's file, or its
//! folder, moves to `trash/<key>/` under the path it had, such as
//! `trash/20261031T093000Z-en.hello/content/en/posts/hello.md`, so
//! restoring moves it back. `trash/` sits beside `content/`, which is all
//! a build reads, so nothing in it is served.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use chrono::NaiveDateTime;
use khalid_api::content_store::BoxFuture;
use khalid_api::post_store::PostStoreError;
use khalid_api::post_trash::{self, PostTrash, TrashError, Trashed};
use khalid_api::resources::{AdminPost, PostStatus};
use khalid_content::parse_post;
use khalid_core::SitePath;

use super::{Site, SitePosts, blocking, failure};

/// The trash folder at the top of a site.
const TRASH: &str = "trash";

impl Site {
    /// Moves `moved`, post `id`'s file or folder inside the site, to the
    /// trash, keyed by now.
    pub(super) fn keep(&self, moved: &Path, id: &str) -> Result<(), PostStoreError> {
        let rel = moved
            .strip_prefix(&self.folder)
            .map_err(|_| failure("a post lies outside its site"))?;
        let mut to = self.trashed_at(&post_trash::key(self.clock.now(), id));
        to.push(rel);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(PostStoreError::new)?;
        }
        fs::rename(moved, &to).map_err(PostStoreError::new)
    }

    /// Whether `key` is one folder name inside `trash/`, which cannot climb.
    pub(super) fn names_one_copy(&self, key: &str) -> bool {
        let inside = SitePath::within(&self.folder, &format!("{TRASH}/{key}")).is_ok();
        inside && !key.contains('/')
    }

    fn trashed_at(&self, key: &str) -> PathBuf {
        let mut path = self.folder.clone();
        path.extend([TRASH, key]);
        path
    }

    /// The path a copy in the trash had in the site, and its status: a
    /// draft's file is three folders down, a post's file or folder four.
    fn trashed_from(&self, key: &str) -> Result<(PathBuf, PostStatus), TrashError> {
        let top = self.trashed_at(key);
        let mut rel = PathBuf::new();
        let mut drafts = top.clone();
        drafts.push(super::DRAFTS);
        let (depth, status) = if drafts.is_dir() {
            (3, PostStatus::Draft)
        } else {
            (4, PostStatus::Published)
        };
        for _ in 0..depth {
            let mut here = top.clone();
            here.push(&rel);
            let entry = fs::read_dir(&here)
                .ok()
                .and_then(|mut entries| entries.next())
                .and_then(Result::ok)
                .ok_or(TrashError::NotFound)?;
            rel.push(entry.file_name());
        }
        Ok((rel, status))
    }

    /// The copy at `key`, or `None` when nothing is there. A key comes from
    /// a request, so one that is not a single folder name inside `trash/`
    /// names nothing (RULES.md BL-SEC-4).
    fn listed(&self, key: &str) -> Option<Trashed> {
        if !self.names_one_copy(key) {
            return None;
        }
        let (stamp, id) = key.split_once('-')?;
        let at = NaiveDateTime::parse_from_str(stamp, "%Y%m%dT%H%M%SZ").ok()?;
        let (rel, status) = self.trashed_from(key).ok()?;
        let mut file = self.trashed_at(key);
        file.push(&rel);
        if file.is_dir() {
            file.push("index.md");
        }
        let text = fs::read_to_string(&file).ok()?;
        let post = parse_post(&rel, &text).ok()?;
        let title = post.front.title.unwrap_or_default();
        Some(Trashed::listed(key, id, &title, status, at.and_utc()))
    }

    fn trashed_now(&self) -> Result<Vec<Trashed>, PostStoreError> {
        let mut folder = self.folder.clone();
        folder.push(TRASH);
        let Ok(entries) = fs::read_dir(&folder) else {
            return Ok(Vec::new());
        };
        let mut keys = Vec::new();
        for entry in entries {
            let entry = entry.map_err(PostStoreError::new)?;
            keys.extend(entry.file_name().to_str().map(str::to_owned));
        }
        keys.sort_unstable_by(|a, b| b.cmp(a));
        Ok(keys.iter().filter_map(|key| self.listed(key)).collect())
    }

    fn restore_now(&self, key: &str) -> Result<AdminPost, TrashError> {
        let _writing = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let trashed = self.listed(key).ok_or(TrashError::NotFound)?;
        let (rel, _) = self.trashed_from(key)?;
        let mut to = self.folder.clone();
        to.push(&rel);
        if to.exists() || self.find(&trashed.id)?.is_some() {
            return Err(TrashError::Taken { id: trashed.id });
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(PostStoreError::new)?;
        }
        let mut from = self.trashed_at(key);
        from.push(&rel);
        fs::rename(&from, &to).map_err(PostStoreError::new)?;
        fs::remove_dir_all(self.trashed_at(key)).map_err(PostStoreError::new)?;
        let restored = self.get_now(&trashed.id)?;
        restored.ok_or_else(|| failure("a restored post cannot be read back").into())
    }

    fn purge_now(&self, key: &str) -> Result<(), TrashError> {
        let _writing = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        self.listed(key).ok_or(TrashError::NotFound)?;
        fs::remove_dir_all(self.trashed_at(key)).map_err(|e| PostStoreError::new(e).into())
    }
}

impl PostTrash for SitePosts {
    fn trashed(&self) -> BoxFuture<'_, Result<Vec<Trashed>, PostStoreError>> {
        Box::pin(blocking(&self.0, Site::trashed_now))
    }

    fn restore<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<AdminPost, TrashError>> {
        let key = key.to_owned();
        Box::pin(blocking(&self.0, move |site| site.restore_now(&key)))
    }

    fn purge<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), TrashError>> {
        let key = key.to_owned();
        Box::pin(blocking(&self.0, move |site| site.purge_now(&key)))
    }
}
