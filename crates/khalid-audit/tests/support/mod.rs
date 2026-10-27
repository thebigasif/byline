//! A site held in memory for the audit's tests.

use std::collections::BTreeMap;

use khalid_audit::{Answer, Crawl, Limit, Source, Unreached, crawl};
use khalid_core::AbsoluteUrl;

/// The blog's home page in every test.
pub(crate) const HOME: &str = "https://www.example.com/blog/";

/// A site held in memory: each address with its answer, and 404 for the
/// rest.
pub(crate) struct Memory(BTreeMap<String, Answer>);

#[cfg(test)]
impl Memory {
    pub(crate) fn new(pages: &[(&str, Answer)]) -> Self {
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
pub(crate) fn html(body: &str) -> Answer {
    page("<title>A page</title>", body)
}

/// A 200 HTML answer with `head` and `body`.
#[cfg(test)]
pub(crate) fn page(head: &str, body: &str) -> Answer {
    let page =
        format!("<!doctype html><html lang=\"en\"><head>{head}</head><body>{body}</body></html>");
    Answer::new(200, page.into_bytes()).with_header("content-type", "text/html; charset=utf-8")
}

/// `text` as an address.
#[cfg(test)]
pub(crate) fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

/// The crawl of `pages` from [`HOME`].
#[cfg(test)]
pub(crate) async fn crawled(pages: &[(&str, Answer)]) -> Crawl {
    crawl(&Memory::new(pages), &url(HOME), Limit::default()).await
}
