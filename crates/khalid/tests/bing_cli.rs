//! `khalid bing` hands Bing Webmaster Tools the sitemap and prints how Bing
//! crawls the site, with the key from `KHALID_BING_API_KEY` (task C8). Bing
//! is a `wiremock` stand-in reached through the hidden test flags.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{Value, json};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Made for these tests; Bing never issued it.
const KEY: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";

/// Runs `khalid bing` with `args` on the demo site, reaching `bing`, with
/// `key` in the environment when given.
#[cfg(test)]
fn khalid_bing(bing: &MockServer, key: Option<&str>, args: &[&str]) -> Output {
    let mut site = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    site.push("../../fixtures/demo-site");
    let port = bing.address().port();
    let endpoint = format!("http://bing.test:{port}/webmaster/api.svc/json/");
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", "bing.test=127.0.0.1"])
        .args(["--test-bing", &endpoint, "bing"])
        .args(args)
        .env_remove("KHALID_BING_API_KEY");
    if let Some(key) = key {
        command.env("KHALID_BING_API_KEY", key);
    }
    command.output().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_sitemap_index_is_handed_to_bing() {
    let bing = MockServer::start().await;
    let sitemap = "https://www.example.com/blog/sitemap.xml";
    Mock::given(method("POST"))
        .and(path("/webmaster/api.svc/json/SubmitFeed"))
        .and(query_param("apikey", KEY))
        .and(body_json(json!({
            "siteUrl": "https://www.example.com/",
            "feedUrl": sitemap,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "d": null })))
        .expect(2)
        .mount(&bing)
        .await;
    let output = khalid_bing(&bing, Some(KEY), &["sitemap"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    assert!(said.contains(sitemap), "{said}");
    assert!(!said.contains(KEY), "{said}");

    let output = khalid_bing(&bing, Some(KEY), &["sitemap", "--json"]);
    assert_eq!(
        output.stdout,
        format!("{{\"submitted\":\"{sitemap}\"}}\n").as_bytes()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_key_says_where_to_make_one_and_sends_nothing() {
    let bing = MockServer::start().await;
    let output = khalid_bing(&bing, None, &["sitemap"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(said.contains("KHALID_BING_API_KEY"), "{said}");
    assert!(said.contains("API Access"), "{said}");
    assert!(bing.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_bing_refuses_is_explained_without_showing_it() {
    let bing = MockServer::start().await;
    let refused = json!({ "ErrorCode": 3, "Message": "InvalidApiKey" });
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(refused))
        .mount(&bing)
        .await;
    let output = khalid_bing(&bing, Some(KEY), &["sitemap"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(said.contains("does not know the API key"), "{said}");
    assert!(!said.contains(KEY), "{said}");
}

/// Answers `name` with `{"d": list}` for the demo site's address.
#[cfg(test)]
async fn answer(bing: &MockServer, name: &str, list: Value) {
    Mock::given(method("GET"))
        .and(path(format!("/webmaster/api.svc/json/{name}")))
        .and(query_param("siteUrl", "https://www.example.com/"))
        .and(query_param("apikey", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "d": list })))
        .mount(bing)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_report_names_the_last_day_the_sitemaps_and_each_issue() {
    let bing = MockServer::start().await;
    let older = json!({ "Date": "/Date(1793318400000-0000)/", "CrawledPages": 4 });
    let newest = json!({
        "Date": "/Date(1793404800000-0000)/", "CrawledPages": 37, "InIndex": 112,
        "CrawlErrors": 2, "BlockedByRobotsTxt": 1,
    });
    answer(&bing, "GetCrawlStats", json!([older, newest])).await;
    let sitemap = json!({
        "Url": "https://www.example.com/blog/sitemap.xml", "Status": "Success",
        "UrlCount": 24, "LastCrawled": "/Date(1793404800000-0000)/",
    });
    answer(&bing, "GetFeeds", json!([sitemap])).await;
    let issue = json!({
        "Url": "https://www.example.com/blog/old-post/", "HttpCode": 404, "Issues": 4,
    });
    answer(&bing, "GetCrawlIssues", json!([issue])).await;

    let output = khalid_bing(&bing, Some(KEY), &["report"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    for part in [
        "Last crawl day, 2026-10-31: 37 pages fetched, 112 in Bing's index, 2 failed, 1 kept out by robots.txt.",
        "- https://www.example.com/blog/sitemap.xml: Success, 24 addresses, last read 2026-10-31.",
        "- https://www.example.com/blog/old-post/ (404): answered with a 4xx status, such as a missing page.",
    ] {
        assert!(said.contains(part), "{part:?} in:\n{said}");
    }

    let output = khalid_bing(&bing, Some(KEY), &["report", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["last_day"]["crawled_pages"], 37, "{body}");
    assert_eq!(body["sitemaps"][0]["url_count"], 24, "{body}");
    assert_eq!(body["issues"][0]["kinds"], json!(["code_4xx"]), "{body}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_site_bing_has_not_crawled_yet_is_said_so() {
    let bing = MockServer::start().await;
    for name in ["GetCrawlStats", "GetFeeds", "GetCrawlIssues"] {
        answer(&bing, name, json!([])).await;
    }
    let output = khalid_bing(&bing, Some(KEY), &["report"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    for part in [
        "Bing has no crawl days for this site yet.",
        "Run khalid bing sitemap to hand it one.",
        "Bing reports no addresses it had trouble with.",
    ] {
        assert!(said.contains(part), "{part:?} in:\n{said}");
    }
}
