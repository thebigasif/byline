//! Fetching a site's home page and its stylesheets through `Outbound`.
//! Every server is a local `wiremock` stand-in; names resolve through a
//! test table, never real DNS.

use std::fmt::Write as _;
use std::net::{IpAddr, Ipv4Addr};

use khalid_core::AbsoluteUrl;
use khalid_outbound::{Limits, Outbound, OutboundError, Reach, Resolver};
use khalid_theme::{FetchError, MAX_CSS_BYTES, MAX_HTML_BYTES, MAX_STYLESHEETS};
use khalid_theme::{Origin, SkipReason, Skipped, fetch_site};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A client where `example.test`, its `assets.` subdomain and the unrelated
/// `cdn.other.test` all point at the local stand-in server.
#[cfg(test)]
fn client() -> Outbound {
    let loopback = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    let hosts = ["example.test", "assets.example.test", "cdn.other.test"]
        .map(|name| (name.to_owned(), loopback.clone()));
    let resolver = Resolver::for_tests(hosts.into(), Reach::PublicAndLoopback);
    Outbound::with_resolver(Limits::default(), resolver).unwrap()
}

#[cfg(test)]
fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

/// Answers `at` with `status` and `body`.
#[cfg(test)]
async fn serve(server: &MockServer, at: &str, status: u16, body: &str) {
    Mock::given(path(at))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(server)
        .await;
}

#[cfg(test)]
fn home(server: &MockServer) -> AbsoluteUrl {
    url(&format!("http://example.test:{}/", server.address().port()))
}

#[tokio::test]
async fn reads_the_home_page_and_its_same_site_stylesheets_in_page_order() {
    let server = MockServer::start().await;
    let port = server.address().port();
    let html = format!(
        r#"<head><link rel="stylesheet" href="/site.css"><style>body {{ color: #1f2328 }}</style>
<link rel="stylesheet" href="http://assets.example.test:{port}/theme.css"></head>
<body><h1>Acme Analytics</h1></body>"#
    );
    serve(&server, "/", 200, &html).await;
    serve(&server, "/site.css", 200, "a { color: #0b57d0 }").await;
    serve(&server, "/theme.css", 200, ":root { --brand: #e4572e }").await;

    let site = fetch_site(&client(), &home(&server)).await.unwrap();

    assert!(site.html().contains("Acme Analytics"));
    let sheets: Vec<(&Origin, &str)> = site
        .stylesheets()
        .iter()
        .map(|sheet| (sheet.origin(), sheet.css()))
        .collect();
    let site_css = Origin::Linked(url(&format!("http://example.test:{port}/site.css")));
    let theme_css = Origin::Linked(url(&format!("http://assets.example.test:{port}/theme.css")));
    assert_eq!(
        sheets,
        [
            (&site_css, "a { color: #0b57d0 }"),
            (&Origin::Inline, "body { color: #1f2328 }"),
            (&theme_css, ":root { --brand: #e4572e }"),
        ]
    );
    assert_eq!(site.skipped(), []);
}

#[tokio::test]
async fn a_stylesheet_on_another_site_is_never_fetched() {
    let server = MockServer::start().await;
    let other = format!("http://cdn.other.test:{}/t.css", server.address().port());
    serve(
        &server,
        "/",
        200,
        &format!(r#"<link rel=stylesheet href="{other}">"#),
    )
    .await;
    Mock::given(path("/t.css"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let site = fetch_site(&client(), &home(&server)).await.unwrap();

    assert_eq!(site.stylesheets(), []);
    assert_eq!(site.skipped()[0].href(), other);
    assert_eq!(site.skipped()[0].reason(), &SkipReason::OtherSite);
}

#[tokio::test]
async fn stylesheets_past_the_count_cap_are_skipped() {
    let server = MockServer::start().await;
    let mut links = String::new();
    for n in 0..20 {
        write!(links, "<link rel=stylesheet href=/{n}.css>").unwrap();
        serve(&server, &format!("/{n}.css"), 200, "p {}").await;
    }
    serve(&server, "/", 200, &links).await;

    let site = fetch_site(&client(), &home(&server)).await.unwrap();

    assert_eq!(site.stylesheets().len(), MAX_STYLESHEETS);
    assert_eq!(site.skipped().len(), 20 - MAX_STYLESHEETS);
    let too_many = |skip: &Skipped| skip.reason() == &SkipReason::TooMany;
    assert!(site.skipped().iter().all(too_many));
}

#[tokio::test]
async fn a_failing_or_oversized_stylesheet_is_skipped_and_the_rest_are_read() {
    let server = MockServer::start().await;
    let links =
        ["big", "gone", "more"].map(|name| format!("<link rel=stylesheet href=/{name}.css>"));
    serve(&server, "/", 200, &links.concat()).await;
    let big = "a{}".repeat(MAX_CSS_BYTES / 3 - 10);
    serve(&server, "/big.css", 200, &big).await;
    serve(&server, "/gone.css", 404, "").await;
    serve(&server, "/more.css", 200, &"b{}".repeat(100)).await;

    let site = fetch_site(&client(), &home(&server)).await.unwrap();

    assert_eq!(site.stylesheets().len(), 1);
    let reasons: Vec<&SkipReason> = site.skipped().iter().map(Skipped::reason).collect();
    assert_eq!(reasons, [&SkipReason::Status(404), &SkipReason::TooLarge]);
}

#[tokio::test]
async fn a_home_page_that_answers_with_an_error_status_stops_the_sync() {
    let server = MockServer::start().await;
    serve(&server, "/", 503, "").await;

    let error = fetch_site(&client(), &home(&server)).await.unwrap_err();

    assert!(
        matches!(error, FetchError::Status { status: 503 }),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_home_page_larger_than_the_cap_is_refused() {
    let server = MockServer::start().await;
    serve(&server, "/", 200, &"<p>".repeat(MAX_HTML_BYTES / 3 + 1)).await;

    let error = fetch_site(&client(), &home(&server)).await.unwrap_err();

    assert!(matches!(error, FetchError::TooLarge { .. }), "{error:?}");
}

#[tokio::test]
async fn a_private_address_is_refused_before_anything_is_sent() {
    let error = fetch_site(&client(), &url("http://192.168.1.10/"))
        .await
        .unwrap_err();

    let blocked = matches!(
        &error,
        FetchError::Unreachable {
            source: OutboundError::Blocked { .. }
        }
    );
    assert!(blocked, "{error:?}");
}

#[tokio::test]
async fn a_name_that_does_not_resolve_is_unreachable() {
    let error = fetch_site(&client(), &url("http://nowhere.test/"))
        .await
        .unwrap_err();

    assert!(matches!(error, FetchError::Unreachable { .. }), "{error:?}");
}
