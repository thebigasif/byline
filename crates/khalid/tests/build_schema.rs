//! A post's hand-written JSON-LD reaches its page's graph, and a broken
//! one stops the build with a message that names the file, checked
//! through `khalid build` on a copy of the demo site.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The post the tests add the field to.
const POST: &str = "content/en/posts/2026-10-05-fixing-slow-pages/index.md";

/// `rel` under `base`.
#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// Copies the folder `from` into `to`, leaving out derived `data/`.
#[cfg(test)]
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = to.to_path_buf();
        target.push(entry.file_name());
        if entry.path().is_dir() {
            if entry.file_name() != "data" {
                copy(&entry.path(), &target);
            }
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Builds a copy of the demo whose post [`POST`] has `field` in its front
/// matter, into `out`.
#[cfg(test)]
fn build_with(field: &str, out: &Path) -> Output {
    let site = tempfile::tempdir().unwrap();
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    copy(&demo, site.path());
    let text = fs::read_to_string(at(site.path(), POST)).unwrap();
    let text = text.replacen("---\n", &format!("---\n{field}"), 1);
    fs::write(at(site.path(), POST), text).unwrap();
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "build", "--out"])
        .arg(out)
        .output()
        .unwrap()
}

#[test]
fn a_posts_event_preset_reaches_its_json_ld() {
    let preset = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../khalid-seo/tests/fixtures/schema/event.json",
    );
    let json = fs::read_to_string(preset).unwrap().replace('\n', " ");
    let out = tempfile::tempdir().unwrap();
    let output = build_with(&format!("schema: '{json}'\n"), out.path());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    let page = fs::read_to_string(at(out.path(), "fixing-slow-pages/index.html")).unwrap();
    let (_, graph) = page.split_once("application/ld+json").unwrap();
    let (graph, _) = graph.split_once("</script>").unwrap();
    assert!(graph.contains("\"Event\""), "{graph}");
    assert!(graph.contains("Internal links clinic"), "{graph}");
}

#[test]
fn broken_json_ld_stops_the_build_and_names_the_file() {
    let out = tempfile::tempdir().unwrap();
    let output = build_with("schema: '{\"@type\": \"Event\",}'\n", out.path());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("fixing-slow-pages/index.md"), "{stderr}");
    assert!(stderr.contains("schema field"), "{stderr}");
    assert!(!at(out.path(), "fixing-slow-pages").exists());
}
