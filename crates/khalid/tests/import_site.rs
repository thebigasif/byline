//! `khalid import wordpress https://...` reads a live site's public REST
//! API through `Outbound` (KH-API-7, RULES.md BL-RUN-8). A local
//! `wiremock` stand-in plays the `WordPress` site, so nothing is fetched from
//! the internet (RULES.md BL-TST-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The host name the binary resolves to the stand-in, through the test hook.
const HOST: &str = "bakery.test";

const POSTS: &str = r#"[{"id":10,"date":"2024-03-12T09:30:44","slug":"keep-a-starter-alive","status":"publish","type":"post","link":"http://bakery.test/2024/03/keep-a-starter-alive/","title":{"rendered":"How to keep a starter alive"},"content":{"rendered":"<p>Feed it every day.</p>"},"author":2}]"#;

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// A site with the demo's settings and no content yet.
#[cfg(test)]
fn site() -> TempDir {
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    let dir = tempfile::tempdir().unwrap();
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    dir
}

/// A stand-in `WordPress` site, whose `/wp-json/` answers `root_status`.
#[cfg(test)]
async fn wordpress(root_status: u16) -> (MockServer, String) {
    let server = MockServer::start().await;
    let address = format!("http://{HOST}:{}", server.address().port());
    let root = format!(r#"{{"name":"Northfield","url":"{address}","home":"{address}"}}"#);
    let answers = [
        ("/wp-json/", root.as_str()),
        ("/wp-json/wp/v2/posts", POSTS),
        ("/wp-json/wp/v2/pages", "[]"),
        (
            "/wp-json/wp/v2/users",
            r#"[{"id":2,"name":"Mara Lind","slug":"mara-lind"}]"#,
        ),
        ("/wp-json/wp/v2/categories", "[]"),
        ("/wp-json/wp/v2/tags", "[]"),
    ];
    for (at, body) in answers {
        let status = if at == "/wp-json/" { root_status } else { 200 };
        Mock::given(path(at))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&server)
            .await;
    }
    (server, address)
}

/// How the binary finds hosts: the stand-in through the test hook, or the
/// real resolver with its private-network guard.
#[derive(Clone, Copy)]
enum Names {
    StandIn,
    Real,
}

/// Runs `khalid --site <site> import wordpress <address> --dry-run`.
#[cfg(test)]
async fn import(site: &Path, address: &str, names: Names) -> (i32, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command
        .arg("--site")
        .arg(site)
        .args(["import", "wordpress", address, "--dry-run"]);
    if let Names::StandIn = names {
        command.arg("--test-hosts").arg(format!("{HOST}=127.0.0.1"));
    }
    let output = tokio::task::spawn_blocking(move || command.output().unwrap())
        .await
        .unwrap();
    let mut printed = String::from_utf8(output.stdout).unwrap();
    printed.push_str(&String::from_utf8(output.stderr).unwrap());
    (output.status.code().unwrap(), printed)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_live_site_is_read_page_by_page_and_planned() {
    let (_server, address) = wordpress(200).await;
    let site = site();
    let (code, printed) = import(site.path(), &address, Names::StandIn).await;
    assert_eq!(code, 0, "{printed}");
    assert!(
        printed.contains("Coming in: 1 post, 0 drafts, 0 pages, 1 author"),
        "{printed}"
    );
    assert!(printed.contains("Slugs kept: 1."), "{printed}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_site_without_a_rest_api_imports_nothing() {
    let (_server, address) = wordpress(404).await;
    let site = site();
    let (code, printed) = import(site.path(), &address, Names::StandIn).await;
    assert_eq!(code, 1, "{printed}");
    assert!(
        printed.contains("it has no REST API at /wp-json/"),
        "{printed}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_address_is_refused_by_the_guard() {
    let (server, _) = wordpress(200).await;
    let site = site();
    let private = format!("http://127.0.0.1:{}", server.address().port());
    let (code, printed) = import(site.path(), &private, Names::Real).await;
    assert_eq!(code, 1, "{printed}");
    assert!(
        printed.contains("Khalid could not read the WordPress site"),
        "{printed}"
    );
    assert!(!within(site.path(), "content/en").exists());
}
