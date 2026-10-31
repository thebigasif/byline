//! How pages, snippets and authors are written in their files, for `SiteEntries`.

use chrono::{DateTime, Utc};
use khalid_api::entry_store::EntryStoreError;
use khalid_api::resources::{
    AdminAuthor, AdminPage, AdminSnippet, AuthorEdit, AuthorFile, NewAuthor, NewPage, NewSnippet,
    PageEdit, SnippetEdit,
};
use khalid_content::{AuthorFront, PageFront, SnippetFront};
use khalid_core::{AbsoluteUrl, Locale, Slug};
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

impl EntryFile for AdminSnippet {
    const FOLDER: &'static str = "snippets";

    fn read(
        key: (Locale, Slug),
        text: &PostText,
        changed: DateTime<Utc>,
    ) -> Result<Self, EntryStoreError> {
        let front: SnippetFront = front(text)?;
        let title = front.title.unwrap_or_default();
        Ok(Self::new(key, &title, text.markdown(), changed))
    }

    fn slug_in(text: &PostText) -> Result<Option<Slug>, EntryStoreError> {
        Ok(front::<SnippetFront>(text)?.slug)
    }

    fn slug(new: &NewSnippet) -> &Slug {
        new.slug()
    }

    fn text(new: &NewSnippet) -> PostText {
        PostText::draft(new.title(), new.slug(), None, new.markdown())
    }

    fn edit(text: &mut PostText, edit: &SnippetEdit) {
        if let Some(title) = edit.title() {
            text.set("title", &quoted(title));
        }
        if let Some(markdown) = edit.markdown() {
            text.set_body(markdown);
        }
    }
}

impl EntryFile for AdminAuthor {
    const FOLDER: &'static str = "authors";

    fn read(
        key: (Locale, Slug),
        text: &PostText,
        changed: DateTime<Utc>,
    ) -> Result<Self, EntryStoreError> {
        let front: AuthorFront = front(text)?;
        let file = AuthorFile {
            name: front.name.unwrap_or_default(),
            role: front.role,
            credentials: front.credentials,
            same_as: front.same_as,
            photo: front.photo,
            bio: text.markdown().to_owned(),
        };
        Ok(Self::new(key, file, changed))
    }

    fn slug_in(text: &PostText) -> Result<Option<Slug>, EntryStoreError> {
        Ok(front::<AuthorFront>(text)?.slug)
    }

    fn slug(new: &NewAuthor) -> &Slug {
        new.slug()
    }

    fn text(new: &NewAuthor) -> PostText {
        let file = new.file();
        let mut text = PostText::empty();
        text.set("name", &quoted(&file.name));
        text.set("slug", new.slug().as_str());
        put(&mut text, "role", file.role.as_deref().map(quoted));
        put(
            &mut text,
            "credentials",
            file.credentials.as_deref().map(quoted),
        );
        put(&mut text, "same_as", addresses(&file.same_as));
        put(&mut text, "photo", file.photo.as_deref().map(quoted));
        text.set_body(&file.bio);
        text
    }

    fn edit(text: &mut PostText, edit: &AuthorEdit) {
        if let Some(name) = edit.name() {
            text.set("name", &quoted(name));
        }
        let texts = [
            ("role", edit.role()),
            ("credentials", edit.credentials()),
            ("photo", edit.photo()),
        ];
        for (key, sent) in texts {
            match sent {
                Some("") => put(text, key, None),
                Some(value) => text.set(key, &quoted(value)),
                None => {}
            }
        }
        if let Some(same_as) = edit.same_as() {
            put(text, "same_as", addresses(same_as));
        }
        if let Some(bio) = edit.bio() {
            text.set_body(bio);
        }
    }
}

/// `urls` as a YAML list on one line, or `None` for no addresses.
fn addresses(urls: &[AbsoluteUrl]) -> Option<String> {
    let quoted: Vec<_> = urls.iter().map(|url| quoted(url.as_str())).collect();
    (!quoted.is_empty()).then(|| format!("[{}]", quoted.join(", ")))
}
