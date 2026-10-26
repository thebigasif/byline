//! A written site reads like any other site and is the same everywhere.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment};
use khalid_perf::{Fixture, FixtureError};
use tempfile::TempDir;

/// A fresh folder holding a generated site of `posts` posts.
#[cfg(test)]
fn site(posts: usize) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    Fixture::new(posts).unwrap().write(dir.path()).unwrap();
    dir
}

#[cfg(test)]
fn inside(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

#[cfg(test)]
fn every_file(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let bytes = fs::read(&path).unwrap();
                files.push((path.strip_prefix(dir).unwrap().to_path_buf(), bytes));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn a_written_site_reads_with_its_settings_and_every_post() {
    let dir = site(60);
    let config = Config::load(dir.path(), &Environment::empty()).unwrap();
    let content = read_content(&ContentTree::walk(dir.path()).unwrap()).unwrap();
    assert_eq!(content.posts.len(), 60);
    assert_eq!(content.authors.len(), 4);
    assert_eq!(content.indexes.len(), 1);
    assert_eq!(config.languages().len(), 1);
}

#[test]
fn the_same_fixture_writes_the_same_bytes() {
    let (first, second) = (site(25), site(25));
    let files = every_file(first.path());
    let wanted = 25 + 4 + 3;
    assert_eq!(
        files.len(),
        wanted,
        "posts, authors, settings, categories, index"
    );
    assert_eq!(files, every_file(second.path()));
}

#[test]
fn a_folder_with_files_in_it_is_refused_and_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(inside(dir.path(), "notes.txt"), "keep me").unwrap();
    let result = Fixture::new(5).unwrap().write(dir.path());
    assert!(
        matches!(result, Err(FixtureError::NotEmpty { .. })),
        "{result:?}"
    );
    assert_eq!(every_file(dir.path()).len(), 1);
}

#[test]
fn the_command_writes_a_site_once_and_explains_a_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let target = inside(dir.path(), "site-3");
    let run = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_khalid-perf"))
            .args(["fixture", "3"])
            .arg(&target)
            .output()
            .unwrap()
    };
    assert!(run().status.success());
    assert_eq!(every_file(&target).len(), 3 + 4 + 3);
    let again = run();
    assert!(!again.status.success());
    let message = String::from_utf8_lossy(&again.stderr);
    assert!(message.contains("already holds files"), "{message}");
}

#[test]
fn a_folder_that_cannot_be_made_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = inside(dir.path(), "file");
    fs::write(&blocker, "not a folder").unwrap();
    let result = Fixture::new(5).unwrap().write(&inside(&blocker, "site"));
    assert!(matches!(result, Err(FixtureError::Io { .. })), "{result:?}");
}
