//! [`IndexStatus`]: whether Google has one address in its index, from the
//! URL Inspection API.

use serde::Deserialize;

use crate::SearchConsoleError;

/// What Google says about one address in its index.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct IndexStatus {
    /// Google's verdict.
    pub verdict: Verdict,
    /// Google's own words for the state, such as `Submitted and indexed`
    /// or `Crawled - currently not indexed`.
    #[serde(default)]
    pub coverage_state: String,
    /// When Google last fetched it, as Google writes it, or `None` when it
    /// never has.
    pub last_crawl_time: Option<String>,
}

/// Google's verdict on an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum Verdict {
    /// In the index, and can show in results.
    Pass,
    /// In the index with problems, such as in another way than asked.
    Partial,
    /// Not in the index, and nothing says it should be.
    Neutral,
    /// Not in the index because something is wrong.
    Fail,
    /// Google gave no verdict, or one this version does not know.
    #[serde(other)]
    Unknown,
}

impl IndexStatus {
    /// The status in the URL Inspection answer `body`.
    ///
    /// # Errors
    ///
    /// [`SearchConsoleError::Answer`] when `body` is not the API's JSON.
    pub fn read(body: &[u8]) -> Result<Self, SearchConsoleError> {
        let answer: Answer =
            serde_json::from_slice(body).map_err(|_| SearchConsoleError::Answer)?;
        Ok(answer.inspection_result.index_status_result)
    }

    /// Whether Google has the address in its index.
    pub fn is_indexed(&self) -> bool {
        matches!(self.verdict, Verdict::Pass | Verdict::Partial)
    }
}

/// The URL Inspection answer, of which only the index part is read.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answer {
    inspection_result: Inspection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Inspection {
    index_status_result: IndexStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_index_part_of_an_inspection() {
        let body = br#"{"inspectionResult": {"inspectionResultLink": "https://search.google.com/x",
            "indexStatusResult": {"verdict": "PASS", "coverageState": "Submitted and indexed",
            "lastCrawlTime": "2026-10-20T08:01:02Z", "indexingState": "INDEXING_ALLOWED"}}}"#;
        let status = IndexStatus::read(body).unwrap();
        assert!(status.is_indexed());
        assert_eq!(status.coverage_state, "Submitted and indexed");
        assert_eq!(
            status.last_crawl_time.as_deref(),
            Some("2026-10-20T08:01:02Z")
        );

        let never =
            br#"{"inspectionResult": {"indexStatusResult": {"verdict": "VERDICT_UNSPECIFIED",
            "coverageState": "URL is unknown to Google"}}}"#;
        let status = IndexStatus::read(never).unwrap();
        assert_eq!(
            (status.verdict, status.last_crawl_time),
            (Verdict::Unknown, None)
        );
        assert!(matches!(
            IndexStatus::read(b"{}"),
            Err(SearchConsoleError::Answer)
        ));
    }
}
