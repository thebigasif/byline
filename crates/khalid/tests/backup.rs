//! `khalid backup`: one archive of the site folder, with a safe copy of the
//! database and without what Khalid can make again (plan section 4.1,
//! KH-OPS-2).

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use flate2::read::GzDecoder;
use serde_json::Value;

/// A site with settings, a post, a theme file, a built page, an image size,
/// a library file, an audit record and a migrated database.
#[cfg(test)]
fn site() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    for (file, text) in [
        ("content/en/first.md", "---\ntitle: First\n---\nHello.\n"),
        ("theme/base.css", "body { color: #222; }\n"),
        ("public/index.html", "<p>built</p>\n"),
        ("data/media/3f2a/first-640.webp", "size"),
        ("media/3f2a/crm.jpg", "library"),
        ("data/audit/trend.json", "[]\n"),
    ] {
        let path = within(dir.path(), file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let migrated = khalid(dir.path(), &["db", "migrate"]);
    assert_eq!(
        migrated.status.code(),
        Some(0),
        "{}",
        text(&migrated.stderr)
    );
    dir
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Runs the binary with `args` after `--site <site>`.
#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Every file in the archive at `path`, with its bytes, in archive order.
#[cfg(test)]
fn unpacked(path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut archive = tar::Archive::new(GzDecoder::new(fs::File::open(path).unwrap()));
    archive
        .entries()
        .unwrap()
        .map(|entry| {
            let mut entry = entry.unwrap();
            let name = entry.path().unwrap().to_string_lossy().into_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (name, bytes)
        })
        .collect()
}

#[test]
fn backup_keeps_the_site_and_a_whole_database_and_leaves_out_what_can_be_made_again() {
    let site = site();
    let out = tempfile::tempdir().unwrap();
    let archive = within(out.path(), "blog.tar.gz");
    let output = khalid(site.path(), &["backup", "--out", archive.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.starts_with("Backed up 6 files"), "{said}");
    assert!(said.contains("blog.tar.gz"), "{said}");
    let files = unpacked(&archive);
    let folder = site
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let names: Vec<String> = files
        .iter()
        .map(|(name, _)| name.strip_prefix(&format!("{folder}/")).unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "content/en/first.md",
            "data/audit/trend.json",
            "media/3f2a/crm.jpg",
            "site.toml",
            "theme/base.css",
            "data/site.db",
        ],
        "{said}"
    );
    let database = &files.last().unwrap().1;
    assert!(database.starts_with(b"SQLite format 3\0"));
}

#[test]
fn backup_with_json_names_the_archive_and_counts_the_files() {
    let site = site();
    let out = tempfile::tempdir().unwrap();
    let archive = within(out.path(), "blog.tar.gz");
    let output = khalid(
        site.path(),
        &["--json", "backup", "--out", archive.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["files"], 6, "{json}");
    assert_eq!(json["database"], true, "{json}");
    assert!(json["archive"].as_str().unwrap().ends_with("blog.tar.gz"));
    assert!(json["bytes"].as_u64().unwrap() > 0, "{json}");
}

#[test]
fn backup_never_replaces_a_file() {
    let site = site();
    let out = tempfile::tempdir().unwrap();
    let archive = within(out.path(), "blog.tar.gz");
    fs::write(&archive, "keep me").unwrap();
    let output = khalid(site.path(), &["backup", "--out", archive.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    assert_eq!(message.lines().count(), 3, "{message}");
    assert!(message.contains("already"), "{message}");
    assert_eq!(fs::read_to_string(&archive).unwrap(), "keep me");
}

#[test]
fn backup_names_the_archive_by_the_time_in_the_current_folder_and_skips_itself() {
    let site = site();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .current_dir(site.path())
        .args(["--site", ".", "backup"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let made: Vec<String> = fs::read_dir(site.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tar.gz"))
        .collect();
    assert_eq!(made.len(), 1, "{made:?}");
    assert!(made[0].contains("-backup-20"), "{made:?}");
    let files = unpacked(&within(site.path(), &made[0]));
    assert!(
        files.iter().all(|(name, _)| !name.ends_with(".tar.gz")),
        "the archive holds itself"
    );
}

#[test]
fn a_site_without_a_database_is_backed_up_without_one() {
    let site = site();
    fs::remove_dir_all(within(site.path(), "data")).unwrap();
    let out = tempfile::tempdir().unwrap();
    let archive = within(out.path(), "blog.tar.gz");
    let output = khalid(
        site.path(),
        &["--json", "backup", "--out", archive.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["database"], false, "{json}");
    assert_eq!(json["files"], 4, "{json}");
}

#[test]
fn backup_needs_a_site_folder_with_settings() {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("backup")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("--site"));
    let empty = tempfile::tempdir().unwrap();
    let output = khalid(empty.path(), &["backup"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("site.toml"));
}
