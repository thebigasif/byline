//! What a crawler gets back for one address, and where it comes from.

use std::collections::BTreeMap;
use std::future::Future;

use khalid_core::AbsoluteUrl;

/// One answer to a request: its status, its headers and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl Answer {
    /// An answer with `status` and `body` and no headers.
    pub fn new(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            headers: BTreeMap::new(),
            body,
        }
    }

    /// The same answer with the header `name` set to `value`. Names are
    /// compared without case, as HTTP does.
    #[must_use]
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers
            .insert(name.to_ascii_lowercase(), value.to_owned());
        self
    }

    /// The HTTP status code.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The value of the header `name`, compared without case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// The body as bytes.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Whether the answer is an HTML page, by its content type.
    pub fn is_html(&self) -> bool {
        self.header("content-type")
            .is_some_and(|kind| kind.trim_start().starts_with("text/html"))
    }
}

/// Why an address gave no answer at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Unreached {
    /// It took longer than the time allowed.
    TimedOut,
    /// Its answer was larger than the size allowed.
    TooLarge,
    /// The address points into a private network, so it was not asked.
    Refused,
    /// The host could not be found, or the connection failed.
    Failed,
}

/// Whether a source's answers carry a web server's headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Headers {
    /// Every header, as a server sends them.
    Sent,
    /// Only `Content-Type` and `Location`, as a folder of files can give.
    Absent,
}

/// Where a crawl gets its answers: a built folder, a running server or a
/// test table. It answers one address at a time and never follows a
/// redirect itself, so the crawl sees each hop.
pub trait Source {
    /// Whether the answers carry the headers a web server sends, such as
    /// `Strict-Transport-Security`. A folder cannot, so the checks on them
    /// run only on a crawl over HTTP.
    const HEADERS: Headers = Headers::Absent;

    /// The answer for `url`, or why there is none.
    ///
    /// # Errors
    ///
    /// Returns an [`Unreached`] when nothing answered.
    fn fetch(&self, url: &AbsoluteUrl) -> impl Future<Output = Result<Answer, Unreached>>;
}
