//! The authors' files of a Ghost import, and where every old address of
//! the Ghost site leads.

use std::fs;
use std::path::Path;

use khalid_core::{AbsoluteUrl, Config, Environment};
use khalid_import::ghost::{MapSource, Plan, REDIRECTS_CSV, plan, read_export};
use khalid_import::{MapTarget, MoveStatus};
use khalid_routes::Routes;

/// The plan of the fixture from the Ghost site at `old` into a blog at
/// `northfield.example.com`, mounted at `mount`.
#[cfg(test)]
fn planned(old: &str, mount: &str) -> Plan {
    let toml = format!(
        "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"{mount}\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n"
    );
    let site = Config::from_toml(Path::new("site.toml"), &toml, &Environment::empty()).unwrap();
    let dir = env!("CARGO_MANIFEST_DIR");
    let text = fs::read_to_string(format!("{dir}/fixtures/ghost/ghost-5.json")).unwrap();
    let old = AbsoluteUrl::parse(old).unwrap();
    plan(
        &read_export(&text).unwrap(),
        &old,
        &site,
        &Routes::default(),
    )
}

#[cfg(test)]
fn file<'a>(plan: &'a Plan, path: &str) -> &'a str {
    let found = plan.files.iter().find(|file| file.path == Path::new(path));
    found.map_or_else(|| panic!("no {path}"), |file| file.text.as_str())
}

#[test]
fn every_author_of_an_imported_post_gets_a_file_with_their_profiles() {
    let plan = planned("https://northfield.example.com", "/blog/");
    assert_eq!(plan.authors, 2);
    assert_eq!(
        file(&plan, "content/en/authors/mara.md"),
        "---\n\
         name: \"Mara Lind\"\n\
         slug: \"mara\"\n\
         same_as:\n  - \"https://maralind.example.org/\"\n  - \"https://x.com/maralind\"\n  - \"https://www.facebook.com/maralind.bakes\"\n\
         ---\n\
         \n\
         Mara bakes the rye.\n"
    );
    assert_eq!(
        file(&plan, "content/en/authors/jonas.md"),
        "---\nname: \"Jonas Berg\"\nslug: \"jonas\"\n---\n\n"
    );
}

#[test]
fn every_old_address_of_a_live_post_page_and_author_has_one_entry() {
    let plan = planned("https://northfield.example.com", "/blog/");
    let new = "https://northfield.example.com/blog";
    let entries: Vec<(&str, &MapTarget, &MapSource)> = plan
        .redirects
        .iter()
        .map(|entry| (entry.from.as_str(), &entry.to, &entry.source))
        .collect();
    let moved = |to: &str| MapTarget::Moved {
        to: format!("{new}{to}"),
        status: MoveStatus::Permanent,
    };
    let post = |id: &str| MapSource::Post(id.to_owned());
    let author = |slug: &str| MapSource::Author(slug.to_owned());
    assert_eq!(
        entries,
        [
            (
                "/keep-a-starter-alive/",
                &moved("/keep-a-starter-alive/"),
                &post("6650a1f2c3d4e5f6a7b8c901")
            ),
            (
                "/about/",
                &moved("/about/"),
                &post("6650a1f2c3d4e5f6a7b8c902")
            ),
            ("/author/mara/", &moved("/authors/mara/"), &author("mara")),
            (
                "/author/jonas/",
                &moved("/authors/jonas/"),
                &author("jonas")
            ),
        ]
    );
    let csv = file(&plan, REDIRECTS_CSV);
    assert!(
        csv.starts_with("from,to,status,match\n/keep-a-starter-alive/,"),
        "{csv}"
    );
    assert_eq!(csv.lines().count(), 5, "{csv}");
}

#[test]
fn an_address_that_stays_the_same_is_kept_and_a_new_host_gets_full_addresses() {
    let root = planned("https://northfield.example.com", "/");
    assert_eq!(root.redirects[0].to, MapTarget::Kept);
    assert!(!file(&root, REDIRECTS_CSV).contains("/keep-a-starter-alive/,"));
    let moved = planned("https://old-bakery.example.org/journal/", "/");
    assert_eq!(moved.redirects[0].from, "/journal/keep-a-starter-alive/");
    assert_eq!(
        moved.redirects[0].to,
        MapTarget::Moved {
            to: "https://northfield.example.com/keep-a-starter-alive/".to_owned(),
            status: MoveStatus::Permanent,
        }
    );
    assert_eq!(moved.redirects[2].from, "/journal/author/mara/");
}
