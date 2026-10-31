//! `khalid audit` fires `audit.finished` to a stand-in receiver on
//! loopback once its run is kept, signed with the webhook's secret, with
//! the score and counts it printed (audit brief G25, KH-API-3).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use khalid_core::{AbsoluteUrl, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::{Event, SIGNATURE_HEADER, SigningKey, Verifier};
use serde_json::Value;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread")]
async fn an_audit_fires_audit_finished_with_the_score_it_printed() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let site = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, site.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut file = site.path().to_path_buf();
    file.push("data");
    fs::create_dir_all(&file).unwrap();
    file.push("site.db");
    let db = Database::open(&file).await.unwrap();
    db.migrate().await.unwrap();
    let id = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let port = receiver.address().port();
    let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/audit.finished")).unwrap();
    let table = WebhookTable::new(db.clone(), id, SystemClock);
    let (_, secret) = table.create(&url, Event::AuditFinished).await.unwrap();
    db.close().await;

    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap()])
        .args(["--test-hosts", "hooks.test=127.0.0.1"])
        .args(["audit", "--offline", "--json"])
        .output()
        .unwrap();
    let printed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(printed["score"].is_u64(), "{printed}");

    let hooks = receiver.received_requests().await.unwrap();
    assert_eq!(hooks.len(), 1, "sent before the command ended");
    let hook = &hooks[0];
    let signature = hook.headers[SIGNATURE_HEADER].to_str().unwrap();
    let verifier = Verifier::new(SigningKey::try_new(&secret).unwrap());
    verifier
        .verify(signature, &hook.body, &SystemClock)
        .unwrap();
    let body: Value = serde_json::from_slice(&hook.body).unwrap();
    assert_eq!(body["event"], "audit.finished", "{body}");
    for field in ["score", "pages", "errors", "warnings", "notices"] {
        assert_eq!(body["audit"][field], printed[field], "{field}: {body}");
    }
}
