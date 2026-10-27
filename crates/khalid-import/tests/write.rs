//! Writing a plan into a site folder: every file, atomically, and nothing
//! when a file would be overwritten.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_core::{Config, Environment};
use khalid_import::wordpress::{Plan, WriteError, plan, read_wxr};
use khalid_routes::Routes;

#[cfg(test)]
fn planned() -> Plan {
    let dir = env!("CARGO_MANIFEST_DIR");
    let text = fs::read_to_string(format!("{dir}/fixtures/wordpress/basic.xml")).unwrap();
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    plan(&read_wxr(&text).unwrap(), None, &site, &Routes::default())
}

#[cfg(test)]
fn inside(root: &Path, relative: &Path) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push(relative);
    path
}

#[test]
fn every_planned_file_is_written_with_its_text() {
    let site = tempfile::tempdir().unwrap();
    let plan = planned();
    let written = plan.write(site.path()).unwrap();
    assert_eq!(written.len(), plan.files.len());
    for file in &plan.files {
        let on_disk = fs::read_to_string(inside(site.path(), &file.path)).unwrap();
        assert_eq!(on_disk, file.text, "{}", file.path.display());
    }
    let posts = inside(site.path(), Path::new("content/en/posts"));
    let leftovers: Vec<_> = fs::read_dir(posts)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().ends_with(".partial"))
        .collect();
    assert_eq!(leftovers.len(), 0, "no partial file is left");
}

#[test]
fn nothing_is_written_when_a_file_already_exists() {
    let site = tempfile::tempdir().unwrap();
    fs::write(
        inside(site.path(), Path::new("categories.toml")),
        "# mine\n",
    )
    .unwrap();
    let error = planned().write(site.path()).unwrap_err();
    let WriteError::Exists { paths } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(paths, &[PathBuf::from("categories.toml")]);
    assert_eq!(
        fs::read_to_string(inside(site.path(), Path::new("categories.toml"))).unwrap(),
        "# mine\n"
    );
    assert!(!inside(site.path(), Path::new("content")).exists());
}

#[cfg(unix)]
#[test]
fn a_folder_that_leads_outside_the_site_is_refused() {
    let site = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), inside(site.path(), Path::new("content")))
        .unwrap();
    let error = planned().write(site.path()).unwrap_err();
    assert!(matches!(error, WriteError::Path { .. }), "{error:?}");
    assert_eq!(fs::read_dir(elsewhere.path()).unwrap().count(), 0);
}
