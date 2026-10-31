//! Publishing and unpublishing a post's file. A draft is any post readers
//! cannot see: a file in `drafts/<language>/`, or a post in `content/`
//! with no date, which `khalid build` leaves out. Publishing either one
//! dates it today behind the lint publish gate (ADR
//! `0000-one-draft-model`).

use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use khalid_api::post_store::{PostStoreError, UpdatePostError};
use khalid_api::resources::PostStatus;
use khalid_lint::Verdict;

use super::{Found, Site, failure};
use crate::atomic;
use crate::post_text::PostText;

/// Whether a post in `content/` with `date` is live: only once it has one.
pub(super) fn content_status(date: Option<NaiveDate>) -> PostStatus {
    match date {
        Some(_) => PostStatus::Published,
        None => PostStatus::Draft,
    }
}

impl Site {
    /// Whether `found` is the file in `drafts/` for its slug.
    pub(super) fn in_drafts(&self, found: &Found) -> bool {
        found.path == self.draft_path(&found.locale, &found.slug)
    }

    /// Dates the draft today, runs the lint publish gate on it as it would
    /// go live, and only when the gate lets it through, writes it under
    /// `content/` and removes the draft. An undated post already in
    /// `content/` is dated where it is. Returns where it now lives.
    pub(super) fn publish(
        &self,
        found: &Found,
        text: &mut PostText,
    ) -> Result<PathBuf, UpdatePostError> {
        if !self.in_drafts(found) {
            return self.publish_in_place(found, text);
        }
        let (name, relative) = self.dated(found, text);
        if let Verdict::Refuse { report } = self.gate(found, text, &relative)? {
            return Err(UpdatePostError::Refused(report));
        }
        let mut folder = self.folder.clone();
        folder.push(format!("content/{}/posts", found.locale));
        fs::create_dir_all(&folder).map_err(PostStoreError::new)?;
        atomic::create(&folder, &name, &text.to_text()).map_err(PostStoreError::new)?;
        fs::remove_file(&found.path).map_err(PostStoreError::new)?;
        folder.push(name);
        Ok(folder)
    }

    /// Dates the undated post `found` in `content/` today behind the gate,
    /// keeping its file, and the pictures beside it, where they are.
    fn publish_in_place(
        &self,
        found: &Found,
        text: &mut PostText,
    ) -> Result<PathBuf, UpdatePostError> {
        self.dated(found, text);
        let relative = found
            .path
            .strip_prefix(&self.folder)
            .map_err(PostStoreError::new)?;
        if let Verdict::Refuse { report } = self.gate(found, text, relative)? {
            return Err(UpdatePostError::Refused(report));
        }
        atomic::replace(&found.path, &text.to_text()).map_err(PostStoreError::new)?;
        Ok(found.path.clone())
    }

    /// Takes a live post down: writes `text` as its draft in `drafts/`,
    /// then removes it from `content/`. Its date stays in the front matter
    /// until it is published again. Returns where it now lives.
    pub(super) fn unpublish(
        &self,
        found: &Found,
        text: &PostText,
    ) -> Result<PathBuf, PostStoreError> {
        let path = self.draft_path(&found.locale, &found.slug);
        let folder = path
            .parent()
            .ok_or_else(|| failure("a draft has no folder"))?;
        fs::create_dir_all(folder).map_err(PostStoreError::new)?;
        let name = format!("{}.md", found.slug);
        atomic::create(folder, &name, &text.to_text()).map_err(PostStoreError::new)?;
        fs::remove_file(&found.path).map_err(PostStoreError::new)?;
        Ok(path)
    }

    /// Dates a draft's `text` today, as publishing does, and returns the
    /// file name and site path it would go live at from `drafts/`.
    pub(super) fn dated(&self, found: &Found, text: &mut PostText) -> (String, PathBuf) {
        let today = self.clock.now().date_naive();
        text.set("date", &today.format("%Y-%m-%d").to_string());
        let name = format!("{today}-{}.md", found.slug);
        let relative = PathBuf::from(format!("content/{}/posts/{name}", found.locale));
        (name, relative)
    }
}
