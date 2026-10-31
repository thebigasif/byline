//! The HSTS check, which only a crawl over HTTP can run.

pub mod support;

use khalid_audit::{Answer, Check, Headers, Limit, Source, Thresholds, Unreached, audit, crawl};
use khalid_core::AbsoluteUrl;
use support::{HOME, Memory, crawled, html, url};

/// A site held in memory that answers as a server does, with its headers.
struct Served(Memory);

impl Source for Served {
    const HEADERS: Headers = Headers::Sent;

    fn fetch(&self, url: &AbsoluteUrl) -> impl Future<Output = Result<Answer, Unreached>> {
        self.0.fetch(url)
    }
}

#[cfg(test)]
async fn hsts(pages: &[(&str, Answer)]) -> Vec<String> {
    let site = Served(Memory::new(pages));
    let crawl = crawl(&site, &url(HOME), Limit::default()).await;
    assert_eq!(crawl.headers(), Headers::Sent);
    audit(&crawl, Thresholds::default())
        .iter()
        .filter(|issue| issue.check() == Check::HstsMissing)
        .map(|issue| issue.url().as_str().to_owned())
        .collect()
}

#[tokio::test]
async fn an_https_page_a_server_sends_without_hsts_is_named() {
    assert_eq!(hsts(&[(HOME, html(""))]).await, [HOME]);
    let sent = html("").with_header("strict-transport-security", "max-age=63072000");
    assert_eq!(hsts(&[(HOME, sent)]).await, Vec::<String>::new());
}

#[tokio::test]
async fn a_folder_sends_no_headers_so_hsts_is_not_checked() {
    let crawl = crawled(&[(HOME, html(""))]).await;
    assert_eq!(crawl.headers(), Headers::Absent);
    let checks: Vec<Check> = audit(&crawl, Thresholds::default())
        .iter()
        .map(khalid_audit::Issue::check)
        .collect();
    assert!(!checks.contains(&Check::HstsMissing), "{checks:?}");
}
