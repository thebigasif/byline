//! Signing in to Google with the owner's refresh token, reading the
//! numbers of each page and its queries, and index status (KH-GRO-5).
//! Google is a `wiremock` stand-in answering as its references show.

#![cfg(test)]

use khalid_core::{AbsoluteUrl, SearchConsoleProperty, Secret};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use khalid_search_console::{Credentials, SearchConsole, SearchConsoleError, Window};
use serde_json::json;
use wiremock::matchers::{body_json, body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Made for these tests; Google never issued them.
const SECRET: &str = "made-up-client-secret";
const REFRESH: &str = "1//made-up-refresh-token";

/// A client for the demo site that reaches `server`, and an `Outbound`
/// that may reach it on loopback.
fn client(server: &MockServer) -> (SearchConsole, Outbound) {
    let site = AbsoluteUrl::parse("https://www.example.com/").unwrap();
    let credentials = Credentials {
        client_id: "1234-abcd.apps.googleusercontent.com".to_owned(),
        client_secret: Secret::new(SECRET.to_owned()),
        refresh_token: Secret::new(REFRESH.to_owned()),
    };
    let endpoint = AbsoluteUrl::parse(&format!("{}/", server.uri())).unwrap();
    let console = SearchConsole::new(&SearchConsoleProperty::UrlPrefix(site), credentials)
        .with_endpoint(&endpoint);
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    (console, outbound)
}

#[tokio::test]
async fn the_refresh_token_is_traded_for_an_access_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string(
            "grant_type=refresh_token&client_id=1234-abcd.apps.googleusercontent.com\
             &client_secret=made-up-client-secret&refresh_token=1%2F%2Fmade-up-refresh-token",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "ya29.made-up",
            "expires_in": 3599,
            "scope": "https://www.googleapis.com/auth/webmasters.readonly",
            "token_type": "Bearer",
        })))
        .expect(1)
        .mount(&server)
        .await;
    let (console, outbound) = client(&server);
    let session = console.sign_in(&outbound).await.unwrap();
    assert!(!format!("{session:?}").contains("ya29"));
    assert!(!format!("{console:?}").contains(SECRET));
    assert!(!format!("{console:?}").contains("made-up-refresh"));
}

#[tokio::test]
async fn a_revoked_refresh_token_is_named_and_odd_answers_are_not_read() {
    let server = MockServer::start().await;
    let refused = json!({ "error": "invalid_grant", "error_description": "Token has been expired or revoked." });
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(refused))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>sign in</html>"))
        .mount(&server)
        .await;
    let (console, outbound) = client(&server);
    let error = console.sign_in(&outbound).await.unwrap_err();
    assert!(matches!(error, SearchConsoleError::TokenRefused), "{error}");
    let error = console.sign_in(&outbound).await.unwrap_err();
    assert!(matches!(error, SearchConsoleError::Answer), "{error}");
}

/// The Search Analytics address of the demo site's property.
const ANALYTICS: &str =
    "/webmasters/v3/sites/https%3A%2F%2Fwww.example.com%2F/searchAnalytics/query";

/// Lets `server` grant the access token `ya29.made-up` to any refresh.
#[cfg(test)]
async fn grant(server: &MockServer) {
    let granted = json!({ "access_token": "ya29.made-up", "expires_in": 3599 });
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(granted))
        .mount(server)
        .await;
}

#[tokio::test]
async fn each_page_comes_with_its_numbers_for_the_window() {
    let server = MockServer::start().await;
    grant(&server).await;
    let rows = json!({ "rows": [{
        "keys": ["https://www.example.com/blog/why-links-matter/"],
        "clicks": 120.0, "impressions": 3400.0, "ctr": 0.0353, "position": 7.4,
    }], "responseAggregationType": "byPage" });
    Mock::given(method("POST"))
        .and(path(ANALYTICS))
        .and(header("authorization", "Bearer ya29.made-up"))
        .and(body_json(json!({
            "startDate": "2026-10-01", "endDate": "2026-10-28",
            "dimensions": ["page"], "rowLimit": 25_000,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(rows))
        .expect(1)
        .mount(&server)
        .await;
    let (console, outbound) = client(&server);
    let session = console.sign_in(&outbound).await.unwrap();
    let window = Window::ending("2026-10-28".parse().unwrap(), 28).unwrap();
    let pages = session.pages(window).await.unwrap();
    assert_eq!(pages.len(), 1);
    let page = pages.first().unwrap();
    assert_eq!(page.key, "https://www.example.com/blog/why-links-matter/");
    assert!((page.clicks - 120.0).abs() < f64::EPSILON);
    assert!((page.position - 7.4).abs() < f64::EPSILON);
}

#[tokio::test]
async fn no_rows_is_no_pages_and_a_property_out_of_reach_is_named() {
    let server = MockServer::start().await;
    grant(&server).await;
    let denied = json!({ "error": { "code": 403, "status": "PERMISSION_DENIED" } });
    Mock::given(method("POST"))
        .and(path(ANALYTICS))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(ANALYTICS))
        .respond_with(ResponseTemplate::new(403).set_body_json(denied))
        .mount(&server)
        .await;
    let (console, outbound) = client(&server);
    let session = console.sign_in(&outbound).await.unwrap();
    let window = Window::ending("2026-10-28".parse().unwrap(), 7).unwrap();
    assert_eq!(session.pages(window).await.unwrap(), []);
    let error = session.pages(window).await.unwrap_err();
    assert!(matches!(error, SearchConsoleError::NotAllowed), "{error}");
}

#[tokio::test]
async fn the_queries_behind_one_page_are_asked_for_that_page_only() {
    let server = MockServer::start().await;
    grant(&server).await;
    let page = "https://www.example.com/blog/why-links-matter/";
    let only = json!({ "dimension": "page", "operator": "equals", "expression": page });
    let rows = json!({ "rows": [{ "keys": ["internal links"],
        "clicks": 40.0, "impressions": 900.0, "ctr": 0.044, "position": 5.1 }] });
    Mock::given(method("POST"))
        .and(path(ANALYTICS))
        .and(body_json(json!({
            "startDate": "2026-10-01", "endDate": "2026-10-28",
            "dimensions": ["query"], "dimensionFilterGroups": [{ "filters": [only] }],
            "rowLimit": 10,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(rows))
        .expect(1)
        .mount(&server)
        .await;
    let (console, outbound) = client(&server);
    let session = console.sign_in(&outbound).await.unwrap();
    let window = Window::ending("2026-10-28".parse().unwrap(), 28).unwrap();
    let queries = session.queries(window, page, 10).await.unwrap();
    let keys: Vec<&str> = queries.iter().map(|row| row.key.as_str()).collect();
    assert_eq!(keys, ["internal links"]);
}

#[tokio::test]
async fn every_search_on_every_page_comes_in_one_request() {
    let server = MockServer::start().await;
    grant(&server).await;
    let rows = json!({ "rows": [{
        "keys": ["https://www.example.com/blog/why-links-matter/",
                 "how many internal links should a blog post have"],
        "clicks": 2.0, "impressions": 60.0, "ctr": 0.033, "position": 8.2 }] });
    Mock::given(method("POST"))
        .and(path(ANALYTICS))
        .and(header("authorization", "Bearer ya29.made-up"))
        .and(body_json(json!({
            "startDate": "2026-10-01", "endDate": "2026-10-28",
            "dimensions": ["page", "query"], "rowLimit": 25_000,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(rows))
        .expect(1)
        .mount(&server)
        .await;
    let (console, outbound) = client(&server);
    let session = console.sign_in(&outbound).await.unwrap();
    let window = Window::ending("2026-10-28".parse().unwrap(), 28).unwrap();
    let searches = session.searches(window).await.unwrap();
    let found: Vec<(&str, &str)> = searches
        .iter()
        .map(|row| (row.page.as_str(), row.query.as_str()))
        .collect();
    assert_eq!(
        found,
        [(
            "https://www.example.com/blog/why-links-matter/",
            "how many internal links should a blog post have"
        )]
    );
}

#[tokio::test]
async fn index_status_comes_from_url_inspection_and_its_quota_is_named() {
    let server = MockServer::start().await;
    grant(&server).await;
    let page = "https://www.example.com/blog/why-links-matter/";
    let inspected = json!({ "inspectionResult": { "indexStatusResult": {
        "verdict": "NEUTRAL", "coverageState": "Crawled - currently not indexed" } } });
    Mock::given(method("POST"))
        .and(path("/v1/urlInspection/index:inspect"))
        .and(header("authorization", "Bearer ya29.made-up"))
        .and(body_json(
            json!({ "inspectionUrl": page, "siteUrl": "https://www.example.com/" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(inspected))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let spent = json!({ "error": { "code": 429, "status": "RESOURCE_EXHAUSTED" } });
    Mock::given(method("POST"))
        .and(path("/v1/urlInspection/index:inspect"))
        .respond_with(ResponseTemplate::new(429).set_body_json(spent))
        .mount(&server)
        .await;
    let (console, outbound) = client(&server);
    let session = console.sign_in(&outbound).await.unwrap();
    let page = AbsoluteUrl::parse(page).unwrap();
    let status = session.inspect(&page).await.unwrap();
    assert!(!status.is_indexed());
    assert_eq!(status.coverage_state, "Crawled - currently not indexed");
    let error = session.inspect(&page).await.unwrap_err();
    assert!(matches!(error, SearchConsoleError::Quota), "{error}");
}
