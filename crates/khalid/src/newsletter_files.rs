//! The newsletters a site has, one folder each in `content/newsletters/`
//! (task 5.7). `main` needs no folder. A folder's name is the `list=` a
//! `::newsletter` block gives, and its `welcome.md` is the email a reader
//! gets on confirming (task 5.9).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use khalid_core::{SitePath, Slug};
use khalid_newsletter::{Lists, Welcome};

/// Where the newsletters' folders are, inside the site folder.
const FOLDER: &str = "content/newsletters";

/// `main` and every newsletter with a folder in the site in `folder`. A
/// folder whose name is not a slug is skipped, as no block can name it,
/// and a site without the folder has `main` alone.
pub(crate) fn lists(folder: &Path) -> Lists {
    let Ok(newsletters) = SitePath::within(folder, FOLDER) else {
        return Lists::main_only();
    };
    let Ok(entries) = fs::read_dir(newsletters.as_path()) else {
        return Lists::main_only();
    };
    let names = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            Slug::try_new(&name)
                .ok()
                .filter(|slug| slug.as_str() == name)
        });
    Lists::new(names)
}

/// The welcome email of each newsletter in `lists` with a readable
/// `welcome.md` in its folder in the site in `folder` (task 5.9). A file
/// that is missing, unreadable or without a `# ` subject line sends no
/// welcome.
pub(crate) fn welcomes(folder: &Path, lists: &Lists) -> BTreeMap<Slug, Welcome> {
    lists
        .iter()
        .filter_map(|list| {
            let file = SitePath::within(folder, &format!("{FOLDER}/{list}/welcome.md")).ok()?;
            let text = fs::read_to_string(file.as_path()).ok()?;
            Some((list.clone(), Welcome::read(&text)?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_folder_named_as_a_slug_is_a_newsletter_beside_main() {
        let site = tempfile::tempdir().unwrap();
        let root = site.path().to_str().unwrap();
        for name in ["weekly", "Not A Slug", "launch-notes"] {
            fs::create_dir_all(format!("{root}/{FOLDER}/{name}")).unwrap();
        }
        fs::write(format!("{root}/{FOLDER}/notes.md"), "a file").unwrap();
        let names: Vec<_> = lists(site.path()).iter().map(Slug::to_string).collect();
        assert_eq!(names, ["launch-notes", "main", "weekly"]);
    }

    #[test]
    fn each_readable_welcome_file_is_its_newsletter_s_welcome() {
        let site = tempfile::tempdir().unwrap();
        let root = site.path().to_str().unwrap();
        for (name, text) in [
            ("main", "# Hello\n\nThanks.\n"),
            ("weekly", "No heading.\n"),
        ] {
            fs::create_dir_all(format!("{root}/{FOLDER}/{name}")).unwrap();
            fs::write(format!("{root}/{FOLDER}/{name}/welcome.md"), text).unwrap();
        }
        let found = welcomes(site.path(), &lists(site.path()));
        let names: Vec<_> = found.keys().map(Slug::to_string).collect();
        assert_eq!(names, ["main"]);
    }

    #[test]
    fn a_site_without_the_folder_has_main_alone() {
        let site = tempfile::tempdir().unwrap();
        assert_eq!(lists(site.path()), Lists::main_only());
    }
}
