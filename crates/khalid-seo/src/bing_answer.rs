//! What the Bing Webmaster Tools API answers with: daily crawl counts, the
//! sitemaps Bing holds, and the addresses it had trouble crawling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

/// One day of Bing's crawl of the site, from `GetCrawlStats`. Bing keeps
/// six months and updates them daily.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct CrawlDay {
    /// The day, as the moment Bing gives for it.
    #[serde(deserialize_with = "date")]
    pub date: DateTime<Utc>,
    /// Pages Bing fetched that day.
    #[serde(default)]
    pub crawled_pages: u64,
    /// Pages of the site in Bing's index.
    #[serde(default)]
    pub in_index: u64,
    /// Fetches that failed.
    #[serde(default)]
    pub crawl_errors: u64,
    /// Addresses `robots.txt` kept Bing from.
    #[serde(default)]
    pub blocked_by_robots_txt: u64,
    /// Fetches answered with a 4xx status.
    #[serde(default)]
    pub code_4xx: u64,
    /// Fetches answered with a 5xx status.
    #[serde(default)]
    pub code_5xx: u64,
    /// Pages Bing found malware on.
    #[serde(default)]
    pub contains_malware: u64,
}

/// A sitemap or feed Bing holds for the site, from `GetFeeds`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct BingSitemap {
    /// Its address, as Bing has it.
    pub url: String,
    /// Bing's word for how the last read went, such as `Success`.
    #[serde(default)]
    pub status: String,
    /// The addresses Bing found in it.
    #[serde(default)]
    pub url_count: u64,
    /// When it was submitted.
    #[serde(default, deserialize_with = "maybe_date")]
    pub submitted: Option<DateTime<Utc>>,
    /// When Bing last read it.
    #[serde(default, deserialize_with = "maybe_date")]
    pub last_crawled: Option<DateTime<Utc>>,
}

/// An address Bing had trouble crawling, from `GetCrawlIssues`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct CrawlIssue {
    /// The address, as Bing has it.
    pub url: String,
    /// The HTTP status Bing last got for it.
    #[serde(default)]
    pub http_code: u16,
    /// Links Bing knows that point at it.
    #[serde(default)]
    pub in_links: u64,
    /// Bing's flags, one bit per kind of issue.
    #[serde(default)]
    issues: u32,
}

impl CrawlIssue {
    /// Each kind of issue Bing flagged for the address, in Bing's order.
    pub fn kinds(&self) -> Vec<CrawlIssueKind> {
        CrawlIssueKind::ALL
            .into_iter()
            .filter(|kind| self.issues & kind.flag() != 0)
            .collect()
    }
}

/// One kind of crawl issue, as Bing's `CrawlIssues` flags name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum CrawlIssueKind {
    /// Answered with a permanent redirect.
    Code301,
    /// Answered with a temporary redirect.
    Code302,
    /// Answered with a 4xx status, such as 404.
    Code4xx,
    /// Answered with a 5xx status.
    Code5xx,
    /// Kept from Bing by `robots.txt`.
    BlockedByRobotsTxt,
    /// Bing found malware on the page.
    ContainsMalware,
    /// An address Bing counts as important is kept from it by `robots.txt`.
    ImportantUrlBlockedByRobotsTxt,
    /// The host name could not be looked up.
    DnsErrors,
    /// The server took too long to answer.
    TimeOutErrors,
}

impl CrawlIssueKind {
    /// Every kind, in the order of Bing's flags.
    const ALL: [Self; 9] = [
        Self::Code301,
        Self::Code302,
        Self::Code4xx,
        Self::Code5xx,
        Self::BlockedByRobotsTxt,
        Self::ContainsMalware,
        Self::ImportantUrlBlockedByRobotsTxt,
        Self::DnsErrors,
        Self::TimeOutErrors,
    ];

    /// The bit Bing sets for this kind.
    fn flag(self) -> u32 {
        match self {
            Self::Code301 => 1,
            Self::Code302 => 2,
            Self::Code4xx => 4,
            Self::Code5xx => 8,
            Self::BlockedByRobotsTxt => 16,
            Self::ContainsMalware => 32,
            Self::ImportantUrlBlockedByRobotsTxt => 64,
            Self::DnsErrors => 128,
            Self::TimeOutErrors => 256,
        }
    }
}

/// Reads a date in Microsoft's JSON form, `/Date(1316156400000-0700)/`:
/// milliseconds since 1970 in UTC, then an offset that only says where it
/// was shown, which is ignored.
fn date<'de, D: Deserializer<'de>>(deserializer: D) -> Result<DateTime<Utc>, D::Error> {
    let text = String::deserialize(deserializer)?;
    parse_date(&text).ok_or_else(|| serde::de::Error::custom("not a /Date(...)/ value"))
}

/// Reads a date that may be missing or `null`.
fn maybe_date<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error> {
    let text: Option<String> = Option::deserialize(deserializer)?;
    Ok(text.as_deref().and_then(parse_date))
}

/// The moment in `/Date(<milliseconds><offset>)/`, or `None` for any
/// other text.
fn parse_date(text: &str) -> Option<DateTime<Utc>> {
    let inner = text.strip_prefix("/Date(")?.strip_suffix(")/")?;
    let sign = usize::from(inner.starts_with('-'));
    let digits = inner
        .get(sign..)?
        .find(['+', '-'])
        .map_or(inner.len(), |at| at.saturating_add(sign));
    let millis: i64 = inner.get(..digits)?.parse().ok()?;
    DateTime::from_timestamp_millis(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_microsofts_json_dates() {
        let at = |text| parse_date(text).map(|at| at.to_rfc3339());
        assert_eq!(
            at("/Date(1316156400000-0700)/").as_deref(),
            Some("2011-09-16T07:00:00+00:00")
        );
        assert_eq!(
            at("/Date(0+0100)/").as_deref(),
            Some("1970-01-01T00:00:00+00:00")
        );
        assert_eq!(
            at("/Date(-1000)/").as_deref(),
            Some("1969-12-31T23:59:59+00:00")
        );
        for wrong in ["2011-09-16", "/Date()/", "/Date(abc)/", "/Date(12"] {
            assert_eq!(at(wrong), None, "{wrong}");
        }
    }

    #[test]
    fn reads_microsofts_crawl_issue_sample() {
        let sample = r#"{
            "__type": "UrlWithCrawlIssues:#Microsoft.Bing.Webmaster.Api",
            "HttpCode": 200, "Issues": 32,
            "Url": "http://example.com/url1.htm", "InLinks": 10
        }"#;
        let issue: CrawlIssue = serde_json::from_str(sample).unwrap();
        assert_eq!(issue.url, "http://example.com/url1.htm");
        assert_eq!((issue.http_code, issue.in_links), (200, 10));
        assert_eq!(issue.kinds(), [CrawlIssueKind::ContainsMalware]);
    }

    #[test]
    fn every_flag_is_its_own_bit() {
        let all: u32 = CrawlIssueKind::ALL.iter().map(|kind| kind.flag()).sum();
        assert_eq!(all, 511);
    }
}
