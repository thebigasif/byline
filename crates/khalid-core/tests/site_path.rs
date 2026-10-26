//! `SitePath` keeps every path inside the site folder: by its text when it is
//! built, and by following symlinks when it is resolved for use.

use std::fs;
use std::path::{Component, Path, PathBuf};

use khalid_core::{SitePath, SitePathError};
use proptest::prelude::{prop_assert, proptest};

#[test]
fn accepts_relative_paths_inside_the_site() {
    let root = Path::new("/srv/site");
    for (input, relative) in [
        ("posts/why-links-matter.md", "posts/why-links-matter.md"),
        ("media/hero.jpg", "media/hero.jpg"),
        ("robots.txt", "robots.txt"),
        ("%2e%2e/notes.md", "%2e%2e/notes.md"),
    ] {
        let path = SitePath::within(root, input).unwrap();
        assert_eq!(path.relative(), Path::new(relative), "input {input:?}");
        assert_eq!(path.as_path(), under(root, relative), "input {input:?}");
    }
}

/// Says whether an error is the one a case expects.
type Check = fn(&SitePathError) -> bool;

#[test]
fn refuses_paths_that_could_leave_the_site() {
    let root = Path::new("/srv/site");
    let cases: [(&str, Check); 9] = [
        ("", |e| matches!(e, SitePathError::Empty)),
        ("/etc/passwd", |e| matches!(e, SitePathError::Absolute)),
        ("../secrets.toml", |e| {
            matches!(e, SitePathError::DotSegment { .. })
        }),
        ("posts/../../etc", |e| {
            matches!(e, SitePathError::DotSegment { .. })
        }),
        ("posts/./draft.md", |e| {
            matches!(e, SitePathError::DotSegment { .. })
        }),
        ("posts//draft.md", |e| {
            matches!(e, SitePathError::EmptySegment)
        }),
        ("..\\secrets.toml", |e| {
            matches!(e, SitePathError::ForbiddenCharacter { character: '\\' })
        }),
        ("C:/Windows", |e| {
            matches!(e, SitePathError::ForbiddenCharacter { character: ':' })
        }),
        ("posts/a\0.md", |e| {
            matches!(e, SitePathError::ForbiddenCharacter { character: '\0' })
        }),
    ];
    for (input, is_expected) in cases {
        let error = SitePath::within(root, input).unwrap_err();
        assert!(is_expected(&error), "input {input:?} gave {error:?}");
    }
}

/// `root` with a trusted `relative` path pushed on, for building expectations.
fn under(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push(relative);
    path
}

proptest! {
    #[test]
    fn no_text_builds_a_path_outside_the_root(input in "[a./\\\\:%0\u{0}-]{0,24}") {
        let root = Path::new("/srv/site");
        if let Ok(path) = SitePath::within(root, &input) {
            prop_assert!(path.as_path().starts_with(root), "input {:?}", input);
            prop_assert!(
                path.relative().components().all(|c| matches!(c, Component::Normal(_))),
                "input {:?}", input
            );
        }
    }
}

#[test]
fn resolves_existing_files_and_new_files_in_existing_folders() {
    let site = tempfile::tempdir().unwrap();
    fs::create_dir(under(site.path(), "posts")).unwrap();
    fs::write(under(site.path(), "posts/a.md"), "# A").unwrap();
    let root = fs::canonicalize(site.path()).unwrap();
    for input in ["posts/a.md", "posts/new.md", "drafts/new.md"] {
        let resolved = SitePath::within(site.path(), input)
            .unwrap()
            .resolve()
            .unwrap();
        assert_eq!(resolved, under(&root, input), "input {input:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_symlink_inside_the_site_that_points_outside_is_refused() {
    let site = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), under(site.path(), "media")).unwrap();
    let path = SitePath::within(site.path(), "media/hero.jpg").unwrap();
    assert!(matches!(
        path.resolve(),
        Err(SitePathError::EscapesRoot { .. })
    ));
}

#[cfg(unix)]
#[test]
fn a_symlink_created_after_validation_that_points_outside_is_refused_on_resolve() {
    let site = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = SitePath::within(site.path(), "media/hero.jpg").unwrap();
    assert!(path.resolve().is_ok(), "nothing is linked yet");
    std::os::unix::fs::symlink(outside.path(), under(site.path(), "media")).unwrap();
    assert!(matches!(
        path.resolve(),
        Err(SitePathError::EscapesRoot { .. })
    ));
}

#[cfg(unix)]
#[test]
fn a_symlink_that_stays_inside_the_site_is_allowed() {
    let site = tempfile::tempdir().unwrap();
    fs::create_dir(under(site.path(), "images")).unwrap();
    std::os::unix::fs::symlink(under(site.path(), "images"), under(site.path(), "media")).unwrap();
    let resolved = SitePath::within(site.path(), "media/hero.jpg")
        .unwrap()
        .resolve()
        .unwrap();
    let root = fs::canonicalize(site.path()).unwrap();
    assert_eq!(resolved, under(&root, "images/hero.jpg"));
}
