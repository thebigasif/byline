//! Serves a fixture site from a local `wiremock` server and reads its
//! theme through `Outbound`. Nothing is fetched from the internet.

use std::net::{IpAddr, Ipv4Addr};

use khalid_core::AbsoluteUrl;
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use khalid_theme::{SiteTheme, fetch_site};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Serves `fixtures/sites/<name>/` at `example.test` and reads its theme.
/// Returns the theme and the site's home address.
#[cfg(test)]
pub(crate) async fn sync(name: &str, stylesheet: &str) -> (SiteTheme, String) {
    let folder = format!("{}/fixtures/sites/{name}", env!("CARGO_MANIFEST_DIR"));
    let read = |file: &str| std::fs::read_to_string(format!("{folder}/{file}")).unwrap();
    let server = MockServer::start().await;
    for (at, body) in [("/", read("index.html")), (stylesheet, read("style.css"))] {
        Mock::given(path(at))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
    }
    let hosts = vec![(
        "example.test".to_owned(),
        vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
    )];
    let resolver = Resolver::for_tests(hosts, Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let home = format!("http://example.test:{}", server.address().port());
    let source = fetch_site(&outbound, &url(&format!("{home}/")))
        .await
        .unwrap();
    (SiteTheme::from_source(&source), home)
}

/// Parses an absolute address the test knows is valid.
#[cfg(test)]
pub(crate) fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}
