//! `khalid restore` refuses an archive `khalid backup` did not write, and
//! makes nothing at all when it does (KH-OPS-2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use flate2::Compression;
use flate2::write::GzEncoder;

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Runs `khalid restore <archive> <folder>`.
#[cfg(test)]
fn restore(archive: &Path, folder: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("restore")
        .args([archive, folder])
        .output()
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// An archive holding `files`, each under the name given, as written.
#[cfg(test)]
fn archive_of(path: &Path, files: &[(&str, &str)]) {
    let gzip = GzEncoder::new(fs::File::create(path).unwrap(), Compression::default());
    let mut tar = tar::Builder::new(gzip);
    for (name, body) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(u64::try_from(body.len()).unwrap());
        header.set_mode(0o644);
        // `append_data` refuses `..`, so the name goes in by hand.
        let bytes = header.as_old_mut().name.as_mut_slice();
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        header.set_cksum();
        tar.append(&header, body.as_bytes()).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
}

/// An archive that is not a site backup, or that names a file outside its
/// folder, makes nothing at all, not even a temporary folder.
#[test]
fn restore_refuses_an_archive_that_is_not_a_site_backup() {
    let out = tempfile::tempdir().unwrap();
    for (files, said) in [
        (&[("photos/cat.jpg", "cat")][..], "site.toml"),
        (
            &[("blog/site.toml", "x"), ("blog/../evil.txt", "x")][..],
            "outside",
        ),
        (
            &[("blog/site.toml", "x"), ("other/a.md", "x")][..],
            "outside",
        ),
    ] {
        let archive = within(out.path(), "odd.tar.gz");
        archive_of(&archive, files);
        let restored = within(out.path(), "restored");
        let made = restore(&archive, &restored);
        assert_eq!(made.status.code(), Some(1), "{files:?}");
        let message = text(&made.stderr);
        assert_eq!(message.lines().count(), 3, "{message}");
        assert!(message.contains(said), "{message}");
        assert!(!restored.exists(), "{files:?} made a folder");
        assert!(!within(out.path(), "evil.txt").exists());
        let left: Vec<_> = fs::read_dir(out.path()).unwrap().collect();
        assert_eq!(left.len(), 1, "a temporary folder was left behind");
        fs::remove_file(&archive).unwrap();
    }
}

#[test]
fn restore_says_when_the_archive_is_missing_or_not_an_archive() {
    let out = tempfile::tempdir().unwrap();
    let restored = within(out.path(), "restored");
    let plain = within(out.path(), "plain.tar.gz");
    fs::write(&plain, "not an archive").unwrap();
    for (archive, why) in [
        (
            within(out.path(), "missing.tar.gz"),
            "There is no file there.",
        ),
        (plain, "It is not a .tar.gz archive, or it is damaged."),
    ] {
        let made = restore(&archive, &restored);
        assert_eq!(made.status.code(), Some(1));
        let message = text(&made.stderr);
        assert!(message.contains("could not read the backup"), "{message}");
        assert!(message.contains(why), "{message}");
        assert!(!restored.exists());
    }
}
