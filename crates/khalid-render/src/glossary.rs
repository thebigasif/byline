//! The glossary from A to Z, and the search page.

use serde::Serialize;

use crate::BodyHtml;

/// The glossary: every term, grouped under its first letter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderGlossary {
    /// The heading, shown as the page's one `h1`.
    pub title: String,
    /// Text above the terms, already sanitised.
    pub intro: Option<BodyHtml>,
    /// The letters that have terms, in the language's alphabetical order.
    pub groups: Vec<GlossaryGroup>,
}

/// The terms under one letter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlossaryGroup {
    /// The letter as readers see it, such as `T` or `Ä`.
    pub letter: String,
    /// A short ASCII id for the letter's section, such as `t`, used in
    /// `#letter-t` jump links.
    pub anchor: String,
    /// The terms, in alphabetical order.
    pub terms: Vec<GlossaryTerm>,
}

/// One term as the glossary lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlossaryTerm {
    /// The term, such as `Topical authority`.
    pub term: String,
    /// The term's own page.
    pub href: String,
    /// One sentence that says what it means.
    pub summary: String,
}

/// The search page: the form, and the results when there was a query.
///
/// The form is a plain `GET` form, so search works with scripts off
/// (RULES.md BL-WEB-2). A script may add live results later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderSearch {
    /// The heading, shown as the page's one `h1`.
    pub title: String,
    /// Whether the site can answer a search, or says it cannot.
    pub mode: SearchMode,
    /// Where the form sends the query, such as `/blog/search/`.
    pub action: String,
    /// What the reader searched for, if they did.
    pub query: Option<String>,
    /// What matched, best first. Empty with a query means nothing matched.
    pub results: Vec<SearchResult>,
}

/// Whether a search page can answer a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// A running server answers the form.
    Live,
    /// The site is plain files, which cannot look words up, so the page
    /// shows readers one sentence where the form was, the note a static
    /// build puts in place of any search form (KH-OPS-1). The owner reads
    /// how to turn search on in `khalid build` and `khalid doctor`.
    Locked,
}

/// One page that matched a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchResult {
    /// The page's title.
    pub title: String,
    /// The page's path.
    pub href: String,
    /// The text around what matched, with each matching word marked.
    pub snippet: Vec<Highlight>,
}

/// One run of a search result's snippet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum Highlight {
    /// Text shown as it is.
    Text(String),
    /// A word that matched the search, shown marked.
    Match(String),
}
