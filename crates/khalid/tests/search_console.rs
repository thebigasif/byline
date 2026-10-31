//! `khalid search-console` reads the demo site's Search Console numbers
//! with the Google secrets from the environment (G43, KH-GRO-5). Google is
//! a `wiremock` stand-in reached through the hidden test flags.

#[expect(
    dead_code,
    reason = "these tests copy the demo site and start no server"
)]
mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use chrono::Days;
use khalid_core::{Clock, SystemClock};
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Made for these tests; Google never issued them.
const SECRET: &str = "made-up-client-secret";
const REFRESH: &str = "1//made-up-refresh-token";

const OPT_IN: &str = "\n[search_console]\nclient_id = \"1234-abcd.apps.googleusercontent.com\"\n";

const ANALYTICS: &str =
    "/webmasters/v3/sites/https%3A%2F%2Fwww.example.com%2F/searchAnalytics/query";

/// A copy of the demo site with `extra` added to its `site.toml`.
#[cfg(test)]
fn demo_site(extra: &str) -> TempDir {
    let temp = common::demo_copy();
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, format!("{text}{extra}")).unwrap();
    temp
}

/// Runs `khalid search-console` with `args` on `site`, reaching `google`,
/// with both secrets in the environment when `signed_in`.
#[cfg(test)]
fn khalid(site: &Path, google: &MockServer, signed_in: Option<()>, args: &[&str]) -> Output {
    let port = google.address().port();
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", "google.test=127.0.0.1"])
        .args(["--test-google", &format!("http://google.test:{port}/")])
        .arg("search-console")
        .args(args)
        .env_remove("KHALID_GOOGLE_CLIENT_SECRET")
        .env_remove("KHALID_GOOGLE_REFRESH_TOKEN");
    if signed_in.is_some() {
        command
            .env("KHALID_GOOGLE_CLIENT_SECRET", SECRET)
            .env("KHALID_GOOGLE_REFRESH_TOKEN", REFRESH);
    }
    command.output().unwrap()
}

/// Lets `google` grant an access token to any refresh.
#[cfg(test)]
async fn grant(google: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "access_token": "ya29.t" })))
        .mount(google)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn check_signs_in_and_reads_the_last_28_final_days() {
    let google = MockServer::start().await;
    grant(&google).await;
    let last = SystemClock.now().date_naive() - Days::new(3);
    let first = last - Days::new(27);
    let rows = json!({ "rows": [{ "keys": ["https://www.example.com/blog/a/"],
        "clicks": 4.0, "impressions": 80.0, "ctr": 0.05, "position": 8.0 }] });
    Mock::given(method("POST"))
        .and(path(ANALYTICS))
        .and(body_partial_json(json!({
            "startDate": first.to_string(), "endDate": last.to_string(),
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(rows))
        .mount(&google)
        .await;
    let site = demo_site(OPT_IN);
    let output = khalid(site.path(), &google, Some(()), &["check", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["property"], "https://www.example.com/");
    assert_eq!(
        (&body["first"], &body["pages"]),
        (&json!(first.to_string()), &json!(1))
    );

    let output = khalid(site.path(), &google, Some(()), &["check"]);
    let said = String::from_utf8(output.stdout).unwrap();
    assert!(
        said.starts_with("Khalid can read the Search Console property https://www.example.com/."),
        "{said}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn without_opting_in_or_secrets_nothing_is_sent_and_the_fix_is_named() {
    let google = MockServer::start().await;
    let site = demo_site("");
    let output = khalid(site.path(), &google, Some(()), &["check"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(said.contains("[search_console]"), "{said}");

    let site = demo_site(OPT_IN);
    let output = khalid(site.path(), &google, None, &["check"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(said.contains("KHALID_GOOGLE_REFRESH_TOKEN"), "{said}");
    assert!(google.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_revoked_token_and_a_closed_property_are_explained_without_secrets() {
    let google = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" })))
        .up_to_n_times(1)
        .mount(&google)
        .await;
    grant(&google).await;
    let denied = json!({ "error": { "code": 403, "status": "PERMISSION_DENIED" } });
    Mock::given(method("POST"))
        .and(path(ANALYTICS))
        .respond_with(ResponseTemplate::new(403).set_body_json(denied))
        .mount(&google)
        .await;
    let site = demo_site(OPT_IN);
    let output = khalid(site.path(), &google, Some(()), &["check"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(
        said.contains("no longer accepts the refresh token"),
        "{said}"
    );
    assert!(!said.contains(SECRET) && !said.contains(REFRESH), "{said}");

    let output = khalid(site.path(), &google, Some(()), &["check"]);
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(
        said.contains("may not read the Search Console property https://www.example.com/"),
        "{said}"
    );
}
