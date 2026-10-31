//! The Bing Webmaster Tools client through `Outbound` (task C8). Every
//! server here is a `wiremock` stand-in that answers with the samples in
//! Microsoft's API reference.

#![cfg(test)]

use khalid_core::{AbsoluteUrl, Secret};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use khalid_seo::{BingError, BingWebmaster, CrawlIssueKind};
use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Made for these tests; Bing never issued it.
const KEY: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";
const SITE: &str = "https://www.example.com/";

fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

/// A client for the demo site that talks to `server`, and an `Outbound`
/// that may reach it on loopback.
fn client(server: &MockServer) -> (BingWebmaster, Outbound) {
    let endpoint = url(&format!("{}/webmaster/api.svc/json/", server.uri()));
    let bing = BingWebmaster::new(url(SITE), Secret::new(KEY.to_owned()))
        .unwrap()
        .with_endpoint(&endpoint);
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    (bing, outbound)
}

/// Answers a GET of `method_name` for the demo site with `body`.
async fn answer_get(server: &MockServer, method_name: &str, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path(format!("/webmaster/api.svc/json/{method_name}")))
        .and(query_param("siteUrl", SITE))
        .and(query_param("apikey", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn submits_the_sitemap_for_the_site() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/webmaster/api.svc/json/SubmitFeed"))
        .and(query_param("apikey", KEY))
        .and(body_json(json!({
            "siteUrl": SITE,
            "feedUrl": "https://www.example.com/blog/sitemap.xml",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "d": null })))
        .expect(1)
        .mount(&server)
        .await;
    let (bing, outbound) = client(&server);
    let sitemap = url("https://www.example.com/blog/sitemap.xml");
    bing.submit_sitemap(&outbound, &sitemap).await.unwrap();
}

#[tokio::test]
async fn reads_the_daily_crawl_counts() {
    let server = MockServer::start().await;
    let day = json!({
        "__type": "CrawlStats:#Microsoft.Bing.Webmaster.Api", "AllOtherCodes": 0,
        "BlockedByRobotsTxt": 2, "Code2xx": 9998, "Code4xx": 1, "Code5xx": 1,
        "CrawlErrors": 3, "CrawledPages": 120, "Date": "/Date(1316156400000-0700)/",
        "InIndex": 1000, "InLinks": 2048,
    });
    answer_get(&server, "GetCrawlStats", json!({ "d": [day] })).await;
    let (bing, outbound) = client(&server);
    let days = bing.crawl_stats(&outbound).await.unwrap();
    let [day] = days.as_slice() else {
        panic!("one day: {days:?}");
    };
    assert_eq!(day.date.to_rfc3339(), "2011-09-16T07:00:00+00:00");
    let counts = (day.crawled_pages, day.in_index, day.crawl_errors);
    assert_eq!(counts, (120, 1000, 3));
    assert_eq!(day.blocked_by_robots_txt, 2);
}

#[tokio::test]
async fn reads_the_sitemaps_bing_holds() {
    let server = MockServer::start().await;
    let feed = json!({
        "__type": "Feed:#Microsoft.Bing.Webmaster.Api",
        "Compressed": false, "FileSize": 1024,
        "LastCrawled": "/Date(1315781995040-0700)/", "Status": "Success",
        "Submitted": "/Date(1316213995040-0700)/", "Type": "Sitemap",
        "Url": "https://www.example.com/blog/sitemap.xml", "UrlCount": 1023,
    });
    answer_get(&server, "GetFeeds", json!({ "d": [feed] })).await;
    let (bing, outbound) = client(&server);
    let sitemaps = bing.sitemaps(&outbound).await.unwrap();
    let [sitemap] = sitemaps.as_slice() else {
        panic!("one sitemap: {sitemaps:?}");
    };
    assert_eq!(sitemap.url, "https://www.example.com/blog/sitemap.xml");
    assert_eq!(
        (sitemap.status.as_str(), sitemap.url_count),
        ("Success", 1023)
    );
    let crawled = sitemap.last_crawled.map(|at| at.date_naive().to_string());
    assert_eq!(crawled.as_deref(), Some("2011-09-11"));
}

#[tokio::test]
async fn reads_each_crawl_issue_as_its_kinds() {
    let server = MockServer::start().await;
    let issue = json!({
        "__type": "UrlWithCrawlIssues:#Microsoft.Bing.Webmaster.Api",
        "HttpCode": 404, "Issues": 4 | 16, "InLinks": 10,
        "Url": "https://www.example.com/blog/old-post/",
    });
    answer_get(&server, "GetCrawlIssues", json!({ "d": [issue] })).await;
    let (bing, outbound) = client(&server);
    let issues = bing.crawl_issues(&outbound).await.unwrap();
    let [issue] = issues.as_slice() else {
        panic!("one issue: {issues:?}");
    };
    assert_eq!(issue.http_code, 404);
    let kinds = [CrawlIssueKind::Code4xx, CrawlIssueKind::BlockedByRobotsTxt];
    assert_eq!(issue.kinds(), kinds);
}

#[tokio::test]
async fn each_refusal_is_named_and_never_shows_the_key() {
    let cases = [
        (
            "InvalidApiKey",
            "Bing Webmaster Tools does not know the API key",
        ),
        (
            "NotAuthorized",
            "the site is not verified in Bing Webmaster Tools for this API key",
        ),
        (
            "ThrottleUser",
            "Bing Webmaster Tools answered 400: ThrottleUser",
        ),
    ];
    for (word, said) in cases {
        let server = MockServer::start().await;
        let refused = json!({ "ErrorCode": 3, "Message": word });
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(400).set_body_json(refused))
            .mount(&server)
            .await;
        let (bing, outbound) = client(&server);
        let error = bing.crawl_stats(&outbound).await.unwrap_err();
        assert_eq!(error.to_string(), said);
        for shown in [format!("{error:?}"), format!("{bing:?}")] {
            assert!(!shown.contains(KEY), "{shown}");
        }
    }
}

#[tokio::test]
async fn an_answer_that_is_not_the_apis_json_is_refused() {
    let server = MockServer::start().await;
    answer_get(&server, "GetCrawlStats", json!({ "d": "not a list" })).await;
    let (bing, outbound) = client(&server);
    let error = bing.crawl_stats(&outbound).await.unwrap_err();
    assert!(matches!(error, BingError::Answer), "{error:?}");
}

#[tokio::test]
async fn a_request_that_does_not_complete_never_shows_the_address() {
    let server = MockServer::start().await;
    let (bing, _) = client(&server);
    // The guard refuses loopback, so nothing is sent.
    let outbound = Outbound::new(Limits::default()).unwrap();
    let error = bing.crawl_stats(&outbound).await.unwrap_err();
    assert!(matches!(error, BingError::Request { .. }), "{error:?}");
    assert!(!format!("{error:?}").contains(KEY), "{error:?}");
}

#[test]
fn a_key_with_characters_bing_never_issues_is_refused() {
    let made_for_this_test = "abc 123&admin=1";
    let made = BingWebmaster::new(url(SITE), Secret::new(made_for_this_test.to_owned()));
    let error = made.unwrap_err();
    assert!(matches!(error, BingError::Key), "{error:?}");
    assert!(!format!("{error:?}").contains(made_for_this_test));
}
