//! `khalid bing` hands Bing Webmaster Tools the sitemap and prints how Bing
//! crawls the site, with the key from `KHALID_BING_API_KEY` (task C8). Bing
//! is a `wiremock` stand-in reached through the hidden test flags.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::json;
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
