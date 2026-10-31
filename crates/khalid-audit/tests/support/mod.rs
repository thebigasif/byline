//! A site held in memory for the audit's tests.

use std::collections::BTreeMap;

use khalid_audit::{Answer, Crawl, Limit, Source, Unreached, crawl};
use khalid_core::AbsoluteUrl;

/// The blog's home page in every test.
pub const HOME: &str = "https://www.example.com/blog/";

/// A site held in memory: each address with its answer, and 404 for the
/// rest.
#[derive(Debug)]
pub struct Memory(BTreeMap<String, Answer>);

#[cfg(test)]
impl Memory {
    /// The site with `pages`, each an address and its answer.
    pub fn new(pages: &[(&str, Answer)]) -> Self {
        Self(
            pages
                .iter()
                .map(|(url, answer)| ((*url).to_owned(), answer.clone()))
                .collect(),
        )
    }
}

impl Source for Memory {
    fn fetch(&self, url: &AbsoluteUrl) -> impl Future<Output = Result<Answer, Unreached>> {
        std::future::ready(Ok(self
            .0
            .get(url.as_str())
            .cloned()
            .unwrap_or_else(|| Answer::new(404, Vec::new()))))
    }
}

/// A 200 HTML answer whose body is `body`, with a title.
#[cfg(test)]
pub fn html(body: &str) -> Answer {
    page("<title>A page</title>", body)
}

/// A 200 HTML answer with `head` and `body`.
#[cfg(test)]
pub fn page(head: &str, body: &str) -> Answer {
    let page =
        format!("<!doctype html><html lang=\"en\"><head>{head}</head><body>{body}</body></html>");
    Answer::new(200, page.into_bytes()).with_header("content-type", "text/html; charset=utf-8")
}

/// `text` as an address.
///
/// # Panics
///
/// Panics when `text` is not an address, which only a wrong test does.
#[cfg(test)]
pub fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

/// The crawl of `pages` from [`HOME`].
#[cfg(test)]
pub async fn crawled(pages: &[(&str, Answer)]) -> Crawl {
    crawl(&Memory::new(pages), &url(HOME), Limit::default()).await
}
