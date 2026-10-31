//! A static site (`mode = "static"`, KH-OPS-1): `khalid build` writes the
//! demo site with no form that posts to an address nothing answers, puts a
//! short note in the page's language where each form was, and says which
//! features are off and why.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use scraper::{Html, Selector};
use serde_json::Value;
use tempfile::TempDir;

/// The mount path of the demo site, where the output folder starts.
const MOUNT: &str = "/blog/";

/// A sign-up block, a contact form and gated content, as a writer adds them.
const FORMS: &str = "
::newsletter{label=\"Your email address\" button=\"Get the weekly links\"}

:::form{id=\"contact\" button=\"Send\"}
- text name: Your name
- email reply_to: Your email address
:::

:::gated{note=\"Subscribers read the rest of this guide.\" label=\"Your email address\" button=\"Read the rest\"}
The full sitemap checklist.
:::
";

/// A copy of the demo site in `mode`, with [`FORMS`] at the end of the
/// sitemap post in English and German.
#[cfg(test)]
fn demo_with_forms(mode: &str) -> TempDir {
    let temp = demo(mode);
    for post in [
        "content/en/posts/2026-09-28-what-a-sitemap-does.md",
        "content/de/posts/2026-09-28-was-eine-sitemap-leistet.md",
    ] {
        let path = at(temp.path(), post);
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, format!("{text}{FORMS}")).unwrap();
    }
    temp
}

/// Every HTML file under `folder`, sorted.
#[cfg(test)]
fn html_files(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(dir) = open.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                open.push(path);
            } else if path.extension().is_some_and(|e| e == "html") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every form in the built site whose address no file in `out` answers, as
/// the page and the address. A form to another site is left out: it works
/// on any host.
#[cfg(test)]
fn forms_to_missing_endpoints(out: &Path) -> Vec<(String, String)> {
    let forms = Selector::parse("form").unwrap();
    let mut missing = Vec::new();
    for file in html_files(out) {
        let page = Html::parse_document(&fs::read_to_string(&file).unwrap());
        for form in page.select(&forms) {
            let action = form.value().attr("action").unwrap_or_default();
            if action.starts_with("https://") || action.starts_with("http://") {
                continue;
            }
            let path = action.split(['?', '#']).next().unwrap_or_default();
            let served = path.strip_prefix(MOUNT).is_some_and(|rest| {
                let rest = if rest.is_empty() || rest.ends_with('/') {
                    format!("{rest}index.html")
                } else {
                    rest.to_owned()
                };
                at(out, &rest).is_file()
            });
            if !served {
                let page = file.strip_prefix(out).unwrap().display().to_string();
                missing.push((page, action.to_owned()));
            }
        }
    }
    missing
}

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A copy of the demo site in `mode`.
#[cfg(test)]
fn demo(mode: &str) -> TempDir {
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let settings = at(temp.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let moded = text.replace("mode = \"server\"", &format!("mode = \"{mode}\""));
    fs::write(&settings, moded).unwrap();
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        // A demo built in place keeps image sizes in data/; a copy starts clean.
        if name == "data" {
            continue;
        }
        let target = at(into, name.to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Runs `khalid build` on `site` into `out`, with `--json` first if asked.
#[cfg(test)]
fn build(site: &Path, out: &Path, json: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(json)
        .arg("--site")
        .arg(site)
        .arg("build")
        .arg("--out")
        .arg(out)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn a_static_build_says_which_features_are_off_and_why() {
    let site = demo("static");
    let out = tempfile::tempdir().unwrap();
    let said = String::from_utf8_lossy(&build(site.path(), out.path(), &[]).stdout).into_owned();
    assert!(said.starts_with("Built "), "{said}");
    assert!(
        said.contains("The site is static, so 6 features that need a server are off."),
        "{said}"
    );
    for what in [
        "Comments are off.",
        "Newsletter sign-up is off.",
        "Forms are off.",
        "Search is off.",
        "The admin is not part of the built site.",
        "Scheduled publish is off.",
    ] {
        assert!(said.contains(what), "{what} in {said}");
    }
    assert!(
        said.contains(
            "Scheduled publish is off.\n  A static site changes only when you build it, so a post set for later waits for the next build.\n  Run `khalid build` on a schedule, or set mode = \"server\" in site.toml and run `khalid serve`.\n"
        ),
        "{said}"
    );
}

#[test]
fn a_static_build_lists_the_features_off_in_json() {
    let site = demo("static");
    let out = tempfile::tempdir().unwrap();
    let output = build(site.path(), out.path(), &["--json"]);
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    let off = body["off"].as_array().unwrap();
    let features: Vec<&str> = off
        .iter()
        .map(|lock| lock["feature"].as_str().unwrap())
        .collect();
    assert_eq!(
        features,
        [
            "comments",
            "newsletter",
            "forms",
            "search",
            "admin",
            "scheduled_publish"
        ]
    );
    for lock in off {
        for part in ["what", "why", "fix"] {
            assert!(lock[part].as_str().is_some_and(|s| !s.is_empty()), "{lock}");
        }
    }
}

#[test]
fn a_static_build_of_the_demo_has_no_form_posting_to_a_missing_endpoint() {
    let site = demo_with_forms("static");
    let out = tempfile::tempdir().unwrap();
    build(site.path(), out.path(), &[]);

    assert_eq!(forms_to_missing_endpoints(out.path()), []);
    let post = fs::read_to_string(at(out.path(), "what-a-sitemap-does/index.html")).unwrap();
    for note in [
        "<p class=\"lock locked\" data-feature=\"newsletter\">Signing up by email is not open on this site.</p>",
        "<p class=\"lock locked\" data-feature=\"forms\">This form cannot be sent from this site.</p>",
        "<p class=\"lock locked\" data-feature=\"newsletter\">The rest of this post is for subscribers, and signing up is not open here.</p>",
    ] {
        assert!(post.contains(note), "{note}");
    }
    assert!(!post.contains("Subscribers read the rest of this guide."));
    assert!(!post.contains("The full sitemap checklist."));
    let german =
        fs::read_to_string(at(out.path(), "de/was-eine-sitemap-leistet/index.html")).unwrap();
    assert!(german.contains("Anmeldungen per E-Mail sind auf dieser Website nicht möglich."));
}

#[test]
fn a_server_build_keeps_its_forms_and_lists_nothing_off() {
    let site = demo_with_forms("server");
    let out = tempfile::tempdir().unwrap();
    let output = build(site.path(), out.path(), &["--json"]);
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(body.get("off").is_none(), "{body}");
    let said = String::from_utf8_lossy(&build(site.path(), out.path(), &[]).stdout).into_owned();
    assert!(!said.contains("features that need a server"), "{said}");
    let post = fs::read_to_string(at(out.path(), "what-a-sitemap-does/index.html")).unwrap();
    assert!(post.contains("action=\"/subscribe\""));
    assert!(!post.contains("class=\"lock locked\""));
}
