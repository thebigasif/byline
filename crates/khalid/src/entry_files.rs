//! How pages are written in their files, for `SiteEntries`.

use chrono::{DateTime, Utc};
use khalid_api::entry_store::EntryStoreError;
use khalid_api::resources::{AdminPage, NewPage, PageEdit};
use khalid_content::PageFront;
use khalid_core::{Locale, Slug};
use serde::de::DeserializeOwned;

use crate::post_text::{PostText, quoted};
use crate::settings_text::put;
use crate::site_entries::EntryFile;

/// The front matter of `text`, read with the schema the site's content
/// reader uses, so a file the Admin API reads is one the site shows.
fn front<F: DeserializeOwned + Default>(text: &PostText) -> Result<F, EntryStoreError> {
    let yaml = text.front_matter();
    if yaml.trim().is_empty() {
        return Ok(F::default());
    }
    serde_saphyr::from_str(&yaml).map_err(EntryStoreError::new)
}

impl EntryFile for AdminPage {
    const FOLDER: &'static str = "pages";

    fn read(
        key: (Locale, Slug),
        text: &PostText,
        changed: DateTime<Utc>,
    ) -> Result<Self, EntryStoreError> {
        let front: PageFront = front(text)?;
        let title = front.title.unwrap_or_default();
        let page = Self::new(key, &title, text.markdown(), changed);
        Ok(page.with_description(front.description.as_deref()))
    }

    fn slug_in(text: &PostText) -> Result<Option<Slug>, EntryStoreError> {
        Ok(front::<PageFront>(text)?.slug)
    }

    fn slug(new: &NewPage) -> &Slug {
        new.slug()
    }

    fn text(new: &NewPage) -> PostText {
        let mut text = PostText::draft(new.title(), new.slug(), None, new.markdown());
        put(&mut text, "description", new.description().map(quoted));
        text
    }

    fn edit(text: &mut PostText, edit: &PageEdit) {
        if let Some(title) = edit.title() {
            text.set("title", &quoted(title));
        }
        match edit.description() {
            Some("") => put(text, "description", None),
            Some(description) => text.set("description", &quoted(description)),
            None => {}
        }
        if let Some(markdown) = edit.markdown() {
            text.set_body(markdown);
        }
    }
}
