//! The lock text fixtures (KH-OPS-1): each file in `fixtures/locks/` holds
//! a page, a `<!-- locked -->` line, and the same page as a static site
//! shows it. The file's name starts with the page's language.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_core::Locale;
use khalid_static::Locks;

/// The line between a fixture's page and its locked page.
const LOCKED: &str = "<!-- locked -->\n";

/// Every fixture file, sorted by name.
#[cfg(test)]
fn fixtures() -> Vec<PathBuf> {
    let mut folder = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    folder.push("fixtures");
    folder.push("locks");
    let mut found: Vec<PathBuf> = fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "html"))
        .collect();
    found.sort();
    found
}

#[test]
fn every_fixture_page_locks_to_its_locked_page() {
    let locks = Locks::load().unwrap();
    let fixtures = fixtures();
    assert!(fixtures.len() >= 7, "{fixtures:?}");
    for path in fixtures {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).unwrap();
        let (page, want) = text.split_once(LOCKED).unwrap();
        let language = name.split('-').next().unwrap();
        let locale = Locale::try_new(language).unwrap();
        let got = locks.page(page, &locale).unwrap();
        assert_eq!(got.html, want, "{name}");
    }
}
