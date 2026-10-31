//! The server source asks the server only for the site's own addresses.

use khalid_audit::{Server, Source, Unreached};
use khalid_core::AbsoluteUrl;
use khalid_outbound::{Limits, Outbound};

#[cfg(test)]
fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

#[tokio::test]
async fn an_address_off_the_sites_origin_is_refused_before_asking() {
    let outbound = Outbound::new(Limits::default().with_redirects(0)).unwrap();
    let server = Server::new(
        outbound,
        &url("https://www.example.com/"),
        &url("http://127.0.0.1:9/"),
    );
    for off in [
        "https://www.example.com.evil.test/blog/",
        "https://www.example.com:8443/blog/",
        "http://www.example.com/blog/",
        "https://other.example.org/blog/",
    ] {
        assert_eq!(
            server.fetch(&url(off)).await,
            Err(Unreached::Refused),
            "{off}"
        );
    }
}
