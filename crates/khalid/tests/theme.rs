//! `khalid theme sync`: the main site's look, read through a local `wiremock`
//! stand-in and written to `theme/tokens.toml` (task 3.8, RULES.md BL-CLI-1
//! to BL-CLI-3, BL-TST-3). Nothing is fetched from the internet.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A home page that links one stylesheet on its own site.
const HOME: &str = r#"<!doctype html><html><head><title>Northwind Bakery</title>
<link rel="stylesheet" href="/style.css"></head><body><h1>Bread</h1></body></html>"#;

/// A warm theme whose pale amber link is too faint to read on the page.
const STYLE: &str = "body { background: #fbf8f3; color: #2b2118; }\na { color: #f0c040; }\n";

/// The host name the binary resolves to the stand-in, through the test hook.
const HOST: &str = "bakery.test";

/// The demo site's settings in a fresh folder.
#[cfg(test)]
fn site() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    dir
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Serves `home` at `/` and, when given, `style` at `/style.css`. Returns the
/// server and the home address under [`HOST`].
#[cfg(test)]
async fn stand_in(home: &str, style: Option<&str>) -> (MockServer, String) {
    let server = MockServer::start().await;
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(home))
        .mount(&server)
        .await;
    if let Some(style) = style {
        Mock::given(path("/style.css"))
            .respond_with(ResponseTemplate::new(200).set_body_string(style))
            .mount(&server)
            .await;
    }
    let home = format!("http://{HOST}:{}/", server.address().port());
    (server, home)
}

/// How the binary resolves host names in a test.
#[derive(Clone, Copy)]
enum Names {
    /// Through the system, with the private-network guard as released.
    Real,
    /// [`HOST`] answers on loopback, through the test hook.
    StandIn,
}

/// Runs the binary on `site` with `args`, off the async thread so the
/// stand-in keeps serving.
#[cfg(test)]
async fn khalid(site: &Path, args: &[&str], names: Names) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command.arg("--site").arg(site).args(args);
    if let Names::StandIn = names {
        command.arg("--test-hosts").arg(format!("{HOST}=127.0.0.1"));
    }
    tokio::task::spawn_blocking(move || command.output().unwrap())
        .await
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_writes_the_tokens_says_where_and_explains_each_contrast_fix() {
    let site = site();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    let file = within(site.path(), "theme/tokens.toml");
    assert!(
        said.contains(&format!("into {}.", file.display())),
        "{said}"
    );
    assert!(
        said.contains("The colour of links, #f0c040, was too faint"),
        "{said}"
    );
    let tokens = fs::read_to_string(&file).unwrap();
    assert!(tokens.contains("background = \"#fbf8f3\""), "{tokens}");
    assert!(!tokens.contains("#f0c040"), "{tokens}");
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_with_json_names_the_file_and_each_fix() {
    let site = site();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;

    let output = khalid(
        site.path(),
        &["--json", "theme", "sync", &home],
        Names::StandIn,
    )
    .await;

    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let body = json(&output);
    let file = within(site.path(), "theme/tokens.toml");
    assert_eq!(body["tokens"], file.display().to_string());
    assert_eq!(body["site"], home);
    let fix = &body["contrast_fixes"][0];
    assert_eq!(fix["token"], "colors.link");
    assert_eq!(fix["before"], "#f0c040");
    assert!(fix["after_ratio"].as_f64().unwrap() >= 4.5, "{fix}");
    assert_eq!(fix["meets_target"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_address_is_refused_before_anything_is_sent() {
    let site = site();

    let output = khalid(
        site.path(),
        &["theme", "sync", "http://127.0.0.1:9/"],
        Names::Real,
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("private address"), "{said}");
    assert!(!within(site.path(), "theme/tokens.toml").exists());

    let output = khalid(
        site.path(),
        &["--json", "theme", "sync", "http://10.0.0.1/"],
        Names::Real,
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    let what = json(&output)["error"]["what"].as_str().unwrap().to_owned();
    assert!(what.contains("private address"), "{what}");
}
