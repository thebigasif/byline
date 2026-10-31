//! `khalid init`: a site folder from four answers, or from flags when there is
//! no terminal to ask in (RULES.md BL-CLI-1 to BL-CLI-4, PRD KH-INS-2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_core::{Config, Environment, Mode};
use serde_json::Value;

/// Runs the binary with stdin closed, which is not a terminal.
#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Checks a message the way BL-CLI-3 asks: three short sentences.
fn assert_what_why_fix(stderr: &str) {
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "want what, why, fix:\n{stderr}");
    for line in lines {
        assert!(line.ends_with('.'), "{line:?}");
        assert!(line.split_whitespace().count() < 25, "{line:?}");
        assert!(!line.contains('\u{2014}'), "{line:?}");
    }
}

fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn path(folder: &Path) -> &str {
    folder.to_str().unwrap()
}

#[test]
fn init_with_flags_writes_a_site_that_loads() {
    let temp = tempfile::tempdir().unwrap();
    let site = at(temp.path(), "blog");
    let output = khalid(&[
        "init",
        path(&site),
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
        "--languages",
        "de,en",
        "--mode",
        "static",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("new --draft \"Your title\""), "{said}");
    assert!(said.contains("drafts/"), "{said}");

    let config = Config::load(&site, &Environment::empty()).unwrap();
    assert_eq!(config.address().as_str(), "https://www.example.com/");
    assert_eq!(config.name(), "www.example.com");
    assert_eq!(config.mount(), "/blog/");
    assert_eq!(config.mode(), Mode::Static);
    assert_eq!(config.default_language().as_str(), "de");
    assert_eq!(config.languages().len(), 2);
    let folders = ["theme/templates", "content/de/posts", "content/en/posts"];
    for folder in folders.into_iter().chain(["drafts/de", "drafts/en"]) {
        assert!(at(&site, folder).is_dir(), "{folder} is missing");
    }

    let checked = khalid(&["--site", path(&site)]);
    assert_eq!(checked.status.code(), Some(0), "{}", text(&checked.stderr));
}

#[test]
fn init_says_how_to_sign_in_to_the_admin_of_a_server_site() {
    let temp = tempfile::tempdir().unwrap();
    let site = at(temp.path(), "blog");
    let address = ["--address", "https://www.example.com"];
    let output = khalid(&[&["init", path(&site)], &address[..]].concat());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("staff owner --email"), "{said}");

    let other = at(temp.path(), "static");
    let output = khalid(&[&["init", path(&other), "--mode", "static"], &address[..]].concat());
    assert!(
        !text(&output.stdout).contains("staff owner"),
        "a static site has no admin"
    );
}

#[test]
fn init_with_json_takes_the_defaults_and_lists_what_it_wrote() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stderr), "");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let created = [
        "site.toml",
        "routes.toml",
        "theme/templates/",
        "content/en/posts/",
        "drafts/en/",
    ];
    assert_eq!(
        json,
        serde_json::json!({ "site": path(temp.path()), "created": created })
    );
    let config = Config::load(temp.path(), &Environment::empty()).unwrap();
    assert_eq!(config.mount(), "/");
    assert_eq!(config.mode(), Mode::Server);
    assert_eq!(config.languages().len(), 1);
    assert_eq!(config.default_language().as_str(), "en");
}

#[test]
fn init_needs_the_address_flag() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&["init", path(temp.path())]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert_what_why_fix(&stderr);
    assert!(stderr.contains("--address"), "{stderr}");
    assert!(!at(temp.path(), "site.toml").exists());
}

#[test]
fn init_with_yes_asks_nothing_and_takes_the_flags() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&["init", path(temp.path()), "--yes", "--languages", "fr"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("--address"));

    let output = khalid(&[
        "init",
        path(temp.path()),
        "-y",
        "--address",
        "https://example.org",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stderr), "", "nothing is asked");
}

#[test]
fn a_missing_address_with_json_prints_the_message_as_json() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&["init", path(temp.path()), "--json"]);
    assert_eq!(output.status.code(), Some(2));
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(body["error"]["fix"].as_str().unwrap().contains("--address"));
}

#[test]
fn init_refuses_an_address_that_is_not_a_web_address() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&["init", path(temp.path()), "--address", "example"]);
    assert_eq!(output.status.code(), Some(2));
    assert_what_why_fix(&text(&output.stderr));
}

#[test]
fn init_refuses_a_folder_that_already_has_a_site() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(at(temp.path(), "site.toml"), "name = \"mine\"\n").unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://example.org",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    assert_what_why_fix(&stderr);
    assert!(stderr.contains("already exists"), "{stderr}");
    let kept = fs::read_to_string(at(temp.path(), "site.toml")).unwrap();
    assert_eq!(kept, "name = \"mine\"\n");
    assert!(!at(temp.path(), "content").exists());
}

#[test]
fn init_refuses_a_mount_path_before_writing_anything() {
    let temp = tempfile::tempdir().unwrap();
    let site = at(temp.path(), "blog");
    let output = khalid(&[
        "init",
        path(&site),
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog//posts/",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    for part in ["what", "fix"] {
        let sentence = body["error"][part].as_str().unwrap();
        assert!(sentence.contains("--mount"), "{part}: {sentence}");
    }
    assert!(!site.exists());
}

#[test]
fn init_prints_the_proxy_rule_when_the_mount_is_a_folder() {
    let temp = tempfile::tempdir().unwrap();
    let site = at(temp.path(), "blog");
    let output = khalid(&[
        "init",
        path(&site),
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
        "--stack",
        "nginx",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(
        said.contains("location /blog/ {"),
        "the nginx rule:\n{said}"
    );
    assert!(
        said.contains("proxy_pass "),
        "the rule passes /blog/ to Khalid:\n{said}"
    );
    assert!(
        said.contains("proxy_pass http://127.0.0.1:8080;"),
        "nginx on the same machine reaches `khalid serve`:\n{said}"
    );
    assert!(
        !said.contains("blog.example.com"),
        "no made-up host:\n{said}"
    );
}

#[test]
fn a_hosted_stack_is_told_the_blog_address_in_its_rule_is_a_guess() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
        "--stack",
        "vercel",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(
        said.contains("Change https://blog.example.com in the rule"),
        "{said}"
    );

    let site = at(temp.path(), "given");
    let args = ["init", path(&site), "--address", "https://www.example.com"];
    let rest = ["--mount", "/blog/", "--stack", "vercel", "--blog"];
    let output = khalid(&[&args[..], &rest[..], &["https://news.example.net"]].concat());
    let said = text(&output.stdout);
    assert!(said.contains("https://news.example.net"), "{said}");
    assert!(
        !said.contains("Change "),
        "a given address is no guess:\n{said}"
    );
}

#[test]
fn init_prints_the_webflow_rule_for_a_mounted_webflow_site() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--mount",
        "/notes/",
        "--stack",
        "webflow",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(
        said.contains("url.pathname.startsWith(\"/notes/\")"),
        "the Worker checks the mount:\n{said}"
    );
    assert!(
        said.contains("X-Forwarded-Host"),
        "the Worker keeps the reader's host:\n{said}"
    );
}

#[test]
fn init_prints_the_shopify_rule_for_a_mounted_shopify_site() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--mount",
        "/journal/",
        "--stack",
        "shopify",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains("url.pathname.startsWith(\"/journal/\")"),
        "the Worker checks the mount"
    );
}

#[test]
fn init_with_json_prints_the_rule_and_its_parts() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
        "--stack",
        "caddy",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["proxy"]["stack"], "caddy", "{json}");
    assert_eq!(json["proxy"]["mount"], "/blog/", "{json}");
    assert!(
        json["proxy"]["rule"]
            .as_str()
            .unwrap()
            .contains("reverse_proxy"),
        "{json}"
    );
}

#[test]
fn init_prints_no_rule_when_the_blog_is_the_whole_site() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--stack",
        "nginx",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("proxy").is_none(), "no mount, no rule: {json}");
}

#[test]
fn init_refuses_a_stack_it_does_not_know() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
        "--stack",
        "drupal",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("drupal"), "{stderr}");
    assert!(
        stderr.contains("shopify"),
        "the known stacks are listed: {stderr}"
    );
    assert!(!at(temp.path(), "site.toml").exists());
}
