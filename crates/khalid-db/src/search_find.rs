//! Finding pages in the site search index and ranking them (ADR "Site
//! search is our own inverted index in SQLite").

use khalid_core::Locale;
use khalid_search::{Corpus, FieldCounts, Posting, Query, QueryTerm, rank};

use crate::{Database, DbError, SiteId};

/// The largest character, which no term can start with, so `start` and
/// `start` followed by it bound every term that begins with `start`.
const AFTER_EVERY_TERM: char = char::MAX;

/// What a search found: how many pages match, and the best of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchResults {
    /// How many pages hold every term of the query.
    pub total: usize,
    /// The best pages, best first, as many as were asked for.
    pub hits: Vec<SearchHit>,
}

/// One page a search found, with what its result shows.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchHit {
    /// The page's address.
    pub href: String,
    /// The page's title.
    pub title: String,
    /// The page's body as plain text, to cut a snippet from.
    pub body: String,
}

impl Database {
    /// The pages of `site` in `locale` that hold every term of `query`,
    /// ranked, with the best `limit` of them. Each term reads only its own
    /// rows, so a search costs about the same on ten pages or ten thousand.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when the database cannot be read, and
    /// [`DbError::Unreadable`] when a stored count is negative.
    pub async fn search(
        &self,
        site: SiteId,
        locale: &Locale,
        query: &Query,
        limit: usize,
    ) -> Result<SearchResults, DbError> {
        let (row, tag) = (site.as_row(), locale.as_str());
        let corpus = sqlx::query!(
            "SELECT count(*) AS pages, \
             coalesce(sum(title_words), 0) AS \"title!: i64\", \
             coalesce(sum(heading_words), 0) AS \"headings!: i64\", \
             coalesce(sum(body_words), 0) AS \"body!: i64\" \
             FROM search_pages WHERE site_id = ?1 AND locale = ?2",
            row,
            tag
        )
        .fetch_one(&self.readers)
        .await?;
        let total = counts([corpus.title, corpus.headings, corpus.body])?;
        let corpus = Corpus::new(count(corpus.pages)?, total);
        let mut postings = Vec::with_capacity(query.terms().len());
        for term in query.terms() {
            postings.push(self.postings(site, locale, term).await?);
        }
        let ranked = rank(&corpus, &postings);
        let mut hits = Vec::with_capacity(limit.min(ranked.len()));
        for id in ranked.iter().take(limit) {
            let hit = sqlx::query_as!(
                SearchHit,
                "SELECT href, title, body FROM search_pages WHERE id = ?1",
                id
            )
            .fetch_one(&self.readers)
            .await?;
            hits.push(hit);
        }
        Ok(SearchResults {
            total: ranked.len(),
            hits,
        })
    }

    /// The pages of `site` in `locale` that `term` finds, by page id.
    async fn postings(
        &self,
        site: SiteId,
        locale: &Locale,
        term: &QueryTerm,
    ) -> Result<Vec<Posting<i64>>, DbError> {
        let (row, tag, exact) = (site.as_row(), locale.as_str(), term.term().as_str());
        // With no word being typed, the range from "" to "" holds no term.
        let from = term.start().unwrap_or_default();
        let to = term
            .start()
            .map(|start| format!("{start}{AFTER_EVERY_TERM}"))
            .unwrap_or_default();
        let rows = sqlx::query!(
            "SELECT p.id, p.title_words, p.heading_words, p.body_words, \
             t.in_title, t.in_headings, t.in_body \
             FROM search_terms t JOIN search_pages p ON p.id = t.page_id \
             WHERE t.site_id = ?1 AND p.locale = ?2 \
             AND (t.term = ?3 OR (t.term >= ?4 AND t.term < ?5))",
            row,
            tag,
            exact,
            from,
            to
        )
        .fetch_all(&self.readers)
        .await?;
        rows.into_iter()
            .map(|row| {
                let lengths = counts([row.title_words, row.heading_words, row.body_words])?;
                let found = counts([row.in_title, row.in_headings, row.in_body])?;
                Ok(Posting::new(row.id, lengths, found))
            })
            .collect()
    }
}

/// Stored counts for the title, the headings and the body.
fn counts([title, headings, body]: [i64; 3]) -> Result<FieldCounts, DbError> {
    Ok(FieldCounts::new(
        count(title)?,
        count(headings)?,
        count(body)?,
    ))
}

/// A stored count as the ranking reads it.
fn count(value: i64) -> Result<u32, DbError> {
    u32::try_from(value).map_err(|_| DbError::Unreadable {
        column: "search count",
        value: value.to_string(),
    })
}
