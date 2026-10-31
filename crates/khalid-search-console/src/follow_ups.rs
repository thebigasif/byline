//! [`Search`] and [`follow_ups`]: the long, conversational searches that
//! showed a page, the way people ask an AI assistant or Google's AI Mode,
//! and whether one of the page's headings already answers each (GEO M3).

use serde::Deserialize;

use crate::SearchConsoleError;

/// The fewest words a search needs to read as a conversational question
/// rather than a few keywords. A site may ask for more or fewer.
pub const FOLLOW_UP_WORDS: usize = 8;

/// The share of a search's own words, out of four, a heading must hold to
/// answer it.
const COVERED_QUARTERS: usize = 3;

/// Words that carry no topic, left out when a search meets a heading.
const FILLER: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "can", "do", "does", "for", "from", "how",
    "has", "have", "i", "if", "in", "is", "it", "me", "my", "of", "on", "or", "should", "so",
    "that", "the", "there", "this", "to", "what", "when", "where", "which", "who", "why", "will",
    "with", "you", "your",
];

/// What Google counted for one search on one page in a window.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "Row")]
#[non_exhaustive]
pub struct Search {
    /// The page's address as Google saw it.
    pub page: String,
    /// What the searcher typed.
    pub query: String,
    /// Times a searcher clicked through.
    pub clicks: f64,
    /// Times the page showed for the search.
    pub impressions: f64,
    /// The average position of its best result, 1 being the top.
    pub position: f64,
}

impl Search {
    /// The rows of a Search Analytics answer asked by page and query, in
    /// Google's order, or none when it has none.
    ///
    /// # Errors
    ///
    /// [`SearchConsoleError::Answer`] when `body` is not the API's JSON or
    /// a row lacks its page or its query.
    pub fn read_all(body: &[u8]) -> Result<Vec<Self>, SearchConsoleError> {
        let rows: Rows = serde_json::from_slice(body).map_err(|_| SearchConsoleError::Answer)?;
        Ok(rows.rows)
    }

    /// How many words the search has.
    pub fn words(&self) -> usize {
        self.query.split_whitespace().count()
    }
}

/// The Search Analytics API's answer. `rows` is missing when there are
/// none.
#[derive(Deserialize)]
struct Rows {
    #[serde(default)]
    rows: Vec<Search>,
}

/// One row as Google sends it, keys in the order the dimensions were asked.
#[derive(Deserialize)]
struct Row {
    keys: Vec<String>,
    clicks: f64,
    impressions: f64,
    position: f64,
}

impl TryFrom<Row> for Search {
    type Error = &'static str;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let mut keys = row.keys.into_iter();
        let (Some(page), Some(query)) = (keys.next(), keys.next()) else {
            return Err("a row without its page and its query");
        };
        Ok(Self {
            page,
            query,
            clicks: row.clicks,
            impressions: row.impressions,
            position: row.position,
        })
    }
}

/// The searches among `searches` with at least `words` words, the most
/// shown first, ties in page and then query order so the same numbers
/// always give the same list.
pub fn follow_ups(searches: &[Search], words: usize) -> Vec<Search> {
    let mut found: Vec<Search> = searches
        .iter()
        .filter(|search| search.words() >= words)
        .cloned()
        .collect();
    found.sort_by(|a, b| {
        b.impressions
            .total_cmp(&a.impressions)
            .then_with(|| a.page.cmp(&b.page))
            .then_with(|| a.query.cmp(&b.query))
    });
    found
}

/// The first of `headings` that answers `query`: one holding at least
/// three in four of the search's topic words. A word matches its plural.
/// `None` when no heading does, or the search has no topic words.
pub fn answered_by<'h>(query: &str, headings: &'h [String]) -> Option<&'h str> {
    let asked = topic_words(query);
    if asked.is_empty() {
        return None;
    }
    let needed = (asked.len() * COVERED_QUARTERS).div_ceil(4);
    headings
        .iter()
        .find(|heading| {
            let held = topic_words(heading);
            asked.iter().filter(|word| held.contains(word)).count() >= needed
        })
        .map(String::as_str)
}

/// The words of `text` in lower case, without punctuation, filler or a
/// plural `s`, each once, in order.
fn topic_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '\'') {
        let word = word.trim_matches('\'').to_lowercase();
        if word.is_empty() || FILLER.contains(&word.as_str()) {
            continue;
        }
        let word = match word.strip_suffix('s') {
            Some(stem) if stem.chars().count() > 2 && !stem.ends_with('s') => stem.to_owned(),
            _ => word,
        };
        if !words.contains(&word) {
            words.push(word);
        }
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(page: &str, query: &str, impressions: f64) -> Search {
        Search {
            page: page.to_owned(),
            query: query.to_owned(),
            clicks: 1.0,
            impressions,
            position: 6.0,
        }
    }

    #[test]
    fn reads_page_and_query_keys_and_refuses_a_row_without_both() {
        let body = br#"{"rows": [{"keys": ["https://www.example.com/a/", "how long"],
            "clicks": 3.0, "impressions": 40.0, "ctr": 0.075, "position": 4.5}]}"#;
        let rows = Search::read_all(body).unwrap();
        assert_eq!(
            rows,
            [Search {
                clicks: 3.0,
                position: 4.5,
                ..search("https://www.example.com/a/", "how long", 40.0)
            }]
        );
        assert_eq!(Search::read_all(b"{}").unwrap(), []);
        let one_key = br#"{"rows": [{"keys": ["https://www.example.com/a/"], "clicks": 1.0, "impressions": 1.0, "ctr": 1.0, "position": 1.0}]}"#;
        assert!(matches!(
            Search::read_all(one_key),
            Err(SearchConsoleError::Answer)
        ));
    }

    #[test]
    fn only_long_searches_are_follow_ups_and_the_most_shown_come_first() {
        let rows = [
            search("/a/", "internal links", 900.0),
            search(
                "/a/",
                "how many internal links should a blog post have",
                40.0,
            ),
            search(
                "/b/",
                "what is the best way to link old posts to new ones",
                120.0,
            ),
            search("/b/", "do internal links help a new blog rank", 120.0),
        ];
        let found = follow_ups(&rows, FOLLOW_UP_WORDS);
        let queries: Vec<&str> = found.iter().map(|row| row.query.as_str()).collect();
        assert_eq!(
            queries,
            [
                "do internal links help a new blog rank",
                "what is the best way to link old posts to new ones",
                "how many internal links should a blog post have",
            ]
        );
        assert_eq!(follow_ups(&rows, 11).len(), 1);
    }

    #[test]
    fn a_heading_answers_a_search_that_shares_its_topic_words() {
        let headings = [
            "Why internal links matter".to_owned(),
            "How many internal links does a post need?".to_owned(),
        ];
        assert_eq!(
            answered_by("how many internal links should a blog post have", &headings),
            Some("How many internal links does a post need?")
        );
        assert_eq!(
            answered_by("how many internal links should a blog page have", &headings),
            None,
            "three of five topic words is less than three in four"
        );
        assert_eq!(
            answered_by("why do internal links matter so much", &headings),
            Some("Why internal links matter")
        );
        assert_eq!(answered_by("what is it", &headings), None);
        assert_eq!(answered_by("how to fix redirect chains", &headings), None);
    }
}
