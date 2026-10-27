//! The site search index: pages written at publish time (ADR "Site search
//! is our own inverted index in SQLite").

use std::collections::BTreeSet;

use khalid_core::Locale;
use khalid_search::IndexedText;
use sqlx::{Sqlite, Transaction};

use crate::{Database, DbError, SiteId};

/// One page as the search index stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchPage {
    /// The content file, relative to the site folder, such as
    /// `content/en/posts/hello.md`.
    pub path: String,
    /// The language the page is written in, which a query searches within.
    pub locale: Locale,
    /// The page's address, such as `/blog/hello/`.
    pub href: String,
    /// The title a result shows.
    pub title: String,
    /// The body as plain text, which a result's snippet is cut from.
    pub body: String,
    /// The page's terms, counted per field.
    pub text: IndexedText,
}

impl SearchPage {
    /// The page from the file at `path` in `locale`, answering at `href`,
    /// with the `title` and plain-text `body` a result shows, and its `text`
    /// as the index stores it.
    pub fn new(
        path: &str,
        locale: Locale,
        href: &str,
        title: &str,
        body: &str,
        text: IndexedText,
    ) -> Self {
        Self {
            path: path.to_owned(),
            locale,
            href: href.to_owned(),
            title: title.to_owned(),
            body: body.to_owned(),
            text,
        }
    }
}

impl Database {
    /// Writes `pages` into the search index of `site`, replacing what it
    /// held for their files, and removes the files named in `removed`, in
    /// one transaction. Other pages are left as they are.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when a row cannot be written.
    pub async fn index_search(
        &self,
        site: SiteId,
        pages: &[SearchPage],
        removed: &BTreeSet<String>,
    ) -> Result<(), DbError> {
        let mut tx = self.writer.begin().await?;
        let row = site.as_row();
        let paths = removed.iter().chain(pages.iter().map(|page| &page.path));
        for path in paths {
            sqlx::query!(
                "DELETE FROM search_pages WHERE site_id = ?1 AND path = ?2",
                row,
                path
            )
            .execute(&mut *tx)
            .await?;
        }
        for page in pages {
            insert_page(&mut tx, site, page).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

/// Writes one page and every one of its terms.
async fn insert_page(
    tx: &mut Transaction<'_, Sqlite>,
    site: SiteId,
    page: &SearchPage,
) -> Result<(), DbError> {
    let (row, tag) = (site.as_row(), page.locale.as_str());
    let lengths = page.text.lengths();
    let id = sqlx::query_scalar!(
        "INSERT INTO search_pages (site_id, locale, path, href, title, body, \
         title_words, heading_words, body_words) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) RETURNING id",
        row,
        tag,
        page.path,
        page.href,
        page.title,
        page.body,
        lengths.title,
        lengths.headings,
        lengths.body,
    )
    .fetch_one(&mut **tx)
    .await?;
    for (term, counts) in page.text.terms() {
        let term = term.as_str();
        sqlx::query!(
            "INSERT INTO search_terms (site_id, term, page_id, in_title, in_headings, in_body) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            row,
            term,
            id,
            counts.title,
            counts.headings,
            counts.body,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
