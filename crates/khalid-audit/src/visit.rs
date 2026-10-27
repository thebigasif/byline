//! One address the crawl asked for, and what it found there.

use khalid_core::AbsoluteUrl;

use crate::{Answer, Page, Unreached};

/// What asking for one address gave.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outcome {
    /// An answer that is not a redirect, such as 200 or 404.
    Status(u16),
    /// A redirect with a `Location` the crawl could read.
    Redirect {
        /// The status, such as 301.
        status: u16,
        /// Where it points.
        to: AbsoluteUrl,
    },
    /// No answer at all.
    Unreached(Unreached),
}

impl Outcome {
    /// Whether the address answered 200 to 299.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Status(200..=299))
    }

    /// Whether the address is broken: 400 and above, or no answer.
    pub fn is_broken(&self) -> bool {
        matches!(self, Self::Status(400..) | Self::Unreached(_))
    }
}

/// One address inside the blog the crawl asked for.
#[derive(Debug, Clone)]
pub struct Visit {
    url: AbsoluteUrl,
    depth: u32,
    outcome: Outcome,
    headers: Vec<(String, String)>,
    bytes: usize,
    page: Option<Page>,
}

impl Visit {
    /// The visit to `url`, `depth` clicks from home, that got `answer`.
    pub(crate) fn answered(url: AbsoluteUrl, depth: u32, answer: &Answer) -> Self {
        let status = answer.status();
        let to = answer
            .header("location")
            .filter(|_| (300..400).contains(&status))
            .and_then(|location| url::Url::parse(url.as_str()).ok()?.join(location).ok())
            .and_then(|to| AbsoluteUrl::parse(to.as_str()).ok());
        let outcome = match to {
            Some(to) => Outcome::Redirect { status, to },
            None => Outcome::Status(status),
        };
        let page = (outcome.is_ok() && answer.is_html()).then(|| Page::read(&url, answer.body()));
        Self {
            headers: ["strict-transport-security", "x-robots-tag", "content-type"]
                .into_iter()
                .filter_map(|name| Some((name.to_owned(), answer.header(name)?.to_owned())))
                .collect(),
            bytes: answer.body().len(),
            url,
            depth,
            outcome,
            page,
        }
    }

    /// The visit to `url` that got no answer.
    pub(crate) fn unreached(url: AbsoluteUrl, depth: u32, why: Unreached) -> Self {
        Self {
            url,
            depth,
            outcome: Outcome::Unreached(why),
            headers: Vec::new(),
            bytes: 0,
            page: None,
        }
    }

    /// The address asked for.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// How many clicks from the home page the crawl first found it.
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// What asking for it gave.
    pub fn outcome(&self) -> &Outcome {
        &self.outcome
    }

    /// The value of the response header `name`, kept for the checks:
    /// `strict-transport-security`, `x-robots-tag` and `content-type`.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(kept, _)| kept.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The size of the body, in bytes.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// The page as read, when the answer was a successful HTML page.
    pub fn page(&self) -> Option<&Page> {
        self.page.as_ref()
    }
}
