//! `GET search/?q=` on a running site: the search page with the ranked
//! results of the words in `q`, each with its text and the matching words
//! marked, as plain HTML that needs no script (plan Phase 5, KH-WEB-2).

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Query as Asked;
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum::routing::get;
use khalid_core::{Config, Locale};
use khalid_db::{Database, SearchResults, SiteId};
use khalid_render::{Highlight, SearchResult};
use khalid_search::{Analyzer, Completion, Language, Query, Snippet, SnippetPart};
use khalid_server::{MediaType, SecurityHeaders};
use serde::Deserialize;

use crate::site_pages::SitePages;
use crate::{build_pages, search_page};

/// The longest query read, in bytes. Eight words fit with room to spare,
/// and anything longer is cut, so a pasted page never becomes a search.
const LONGEST_QUERY: usize = 200;

/// How many results one page of search shows.
const RESULTS: usize = 20;

/// How results pages are cached: never kept, since a save changes them.
const NOT_KEPT: HeaderValue = HeaderValue::from_static("no-cache");

/// What a search reads and renders into. Cloning shares it.
#[derive(Debug, Clone)]
pub(crate) struct Search {
    pub(crate) pages: SitePages,
    pub(crate) db: Database,
    pub(crate) site: SiteId,
    pub(crate) security: SecurityHeaders,
}

/// The query string of a search page.
#[derive(Debug, Deserialize)]
struct Words {
    q: Option<String>,
}

/// The routes that answer each language's search page of the site with
/// `config`. Empty unless the site answers searches, so the page from
/// the last render answers in static mode.
pub(crate) fn router(config: &Config, search: Search) -> Router {
    let mut router = Router::new();
    if search_page::mode(config) != khalid_render::SearchMode::Live {
        return router;
    }
    let search = Arc::new(search);
    for locale in config.languages() {
        let path = build_pages::path_of(config, locale, search_page::REST);
        let (search, locale, at) = (Arc::clone(&search), locale.clone(), path.clone());
        let answer = move |Asked(words): Asked<Words>| {
            let (search, locale, at) = (Arc::clone(&search), locale.clone(), at.clone());
            async move { answer(&search, &locale, &at, words.q).await }
        };
        router = router.route(&path, get(answer));
    }
    router
}

/// The results page for `typed` at the search page `path` in `locale`.
async fn answer(
    search: &Search,
    locale: &Locale,
    path: &str,
    typed: Option<String>,
) -> Response<Body> {
    let typed = typed
        .map(|text| cut(text.trim()))
        .filter(|text| !text.is_empty());
    let analyzer = Analyzer::new(Language::of(locale));
    let query = typed
        .as_deref()
        .and_then(|text| Query::parse(&analyzer, text, Completion::WholeWords));
    let mut results = Vec::new();
    if let Some(query) = &query {
        match search.db.search(search.site, locale, query, RESULTS).await {
            Ok(found) => results = shown(&analyzer, query, found),
            Err(error) => {
                tracing::warn!(?error, "the search index could not be read");
                return unavailable(&search.security);
            }
        }
    }
    match search.pages.render_search(path, typed, results) {
        Ok(Some(html)) => page(&search.security, html),
        Ok(None) => unavailable(&search.security),
        Err(message) => {
            tracing::warn!(%message, "a search page could not be rendered");
            unavailable(&search.security)
        }
    }
}

/// Each page `found`, with its text cut around the words of `query`.
fn shown(analyzer: &Analyzer, query: &Query, found: SearchResults) -> Vec<SearchResult> {
    found
        .hits
        .into_iter()
        .map(|hit| {
            let snippet = Snippet::of(analyzer, query, &hit.body);
            SearchResult {
                title: hit.title,
                href: hit.href,
                snippet: snippet.parts().iter().map(highlight).collect(),
            }
        })
        .collect()
}

fn highlight(part: &SnippetPart) -> Highlight {
    match part {
        SnippetPart::Text(text) => Highlight::Text(text.clone()),
        SnippetPart::Match(word) => Highlight::Match(word.clone()),
    }
}

/// The first [`LONGEST_QUERY`] bytes of `text`, ending on a whole letter.
fn cut(text: &str) -> String {
    let mut end = text.len().min(LONGEST_QUERY);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.get(..end).unwrap_or_default().to_owned()
}

/// A results page with the security headers, never cached.
fn page(security: &SecurityHeaders, html: String) -> Response<Body> {
    let mut response = Response::new(Body::from(html));
    let headers = response.headers_mut();
    let html = HeaderValue::from_static(MediaType::Html.as_header());
    headers.insert(header::CONTENT_TYPE, html);
    headers.insert(header::CACHE_CONTROL, NOT_KEPT);
    security.apply(headers);
    response
}

/// 503 while the site cannot answer a search: before the first render, or
/// when the index cannot be read. The reader may try again.
fn unavailable(security: &SecurityHeaders) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    security.apply(response.headers_mut());
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_query_is_cut_on_a_whole_letter() {
        let long = "ä".repeat(150);
        let kept = cut(&long);
        assert_eq!(kept.len(), LONGEST_QUERY);
        assert!(kept.chars().all(|c| c == 'ä'));
        assert_eq!(cut("links"), "links");
    }
}
