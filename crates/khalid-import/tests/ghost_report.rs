//! The dry-run report of a Ghost import, and the import written into a
//! site folder from end to end.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_content::{ContentTree, read_content};
use khalid_core::{AbsoluteUrl, Config, Environment};
use khalid_import::WriteError;
use khalid_import::ghost::{Plan, plan, read_export};
use khalid_routes::Routes;

/// The plan of a Ghost export from `northfield.example.com` into a blog
/// mounted at `/blog/` on the same host.
#[cfg(test)]
fn planned(export: &str) -> Plan {
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    let old = AbsoluteUrl::parse("https://northfield.example.com").unwrap();
    plan(
        &read_export(export).unwrap(),
        &old,
        &site,
        &Routes::default(),
    )
}

/// `rest` inside the folder `dir`.
#[cfg(test)]
fn under(dir: &Path, rest: &str) -> PathBuf {
    let mut path = dir.to_owned();
    path.push(rest);
    path
}

#[cfg(test)]
fn fixture() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    fs::read_to_string(format!("{dir}/fixtures/ghost/ghost-5.json")).unwrap()
}

#[test]
fn the_report_says_what_comes_in_what_is_mapped_and_what_is_not_carried() {
    assert_eq!(
        planned(&fixture()).report().to_string(),
        "This is a dry run: nothing was written.\n\
         Coming in: 1 post, 3 drafts, 1 page and 2 authors.\n\
         Slugs kept: 5. Slugs made for posts that had none: 0.\n\
         Old addresses mapped: 4, of which 4 move and 0 stay the same.\n\
         Tag archives not mapped until posts can hold tags: 2.\n\
         Posts with fields their files cannot hold yet: keep-a-starter-alive (search title, more authors, tags, feature image, Open Graph card, X card); about (canonical address); spring-flour (scheduled time); members-loaf (members only).\n\
         Posts for members only, kept as drafts so nothing paid is published: 1.\n\
         Posts with code injection, which is never carried: 6650a1f2c3d4e5f6a7b8c901.\n\
         Cards with no place in Khalid, left out: 0.\n\
         Posts that could not come in: 0.\n\
         Posts left behind: 0.\n"
    );
}

#[test]
fn the_report_names_changed_slugs_cards_failures_and_posts_left_behind() {
    let text = r#"{"db":[{"data":{"posts":[{"id":"a1","status":"sent"},{"id":"a2","slug":"no-date","status":"published"},{"id":"a3","slug":"Rye_Bread","status":"draft","lexical":"{\"root\":{\"children\":[{\"type\":\"signup\"}]}}"},{"id":"a4","status":"draft","title":"Spelt","lexical":"{\"root\":{\"children\":[{\"type\":\"signup\"},{\"type\":\"product\"}]}}"}]}}]}"#;
    let report = planned(text).report().to_string();
    for line in [
        "Slugs kept: 0. Slugs made for posts that had none: 1.\n",
        "Slugs changed to fit the site's rules: Rye_Bread to rye-bread.\n",
        "Cards with no place in Khalid, left out: signup (2 posts); product (1 post).\n",
        "Posts that could not come in: a2, the post was published but has no date.\n",
        "Posts left behind: a1, a newsletter that was only ever emailed.\n",
    ] {
        assert!(report.contains(line), "{line:?} in {report}");
    }
}

#[test]
fn the_written_site_reads_back_with_every_post_page_and_author() {
    let site = tempfile::tempdir().unwrap();
    let plan = planned(&fixture());
    let written = plan.write(site.path()).unwrap();
    assert_eq!(written.len(), plan.files.len());
    let content = read_content(&ContentTree::walk(site.path()).unwrap()).unwrap();
    let slug = |front: Option<&khalid_core::Slug>| front.map(ToString::to_string);
    assert_eq!(content.posts.len(), 1);
    assert_eq!(
        slug(content.posts[0].front.slug.as_ref()).as_deref(),
        Some("keep-a-starter-alive")
    );
    assert_eq!(content.pages.len(), 1);
    let mut authors: Vec<Option<String>> = content
        .authors
        .iter()
        .map(|author| slug(author.front.slug.as_ref()))
        .collect();
    authors.sort();
    assert_eq!(authors, [Some("jonas".to_owned()), Some("mara".to_owned())]);
    for draft in ["rye-slowly", "spring-flour", "members-loaf"] {
        assert!(
            under(site.path(), &format!("drafts/en/{draft}.md")).is_file(),
            "{draft}"
        );
    }
    let csv = fs::read_to_string(under(site.path(), "import/ghost-redirects.csv")).unwrap();
    assert_eq!(csv.lines().count(), 5);
    let again = plan.write(site.path()).unwrap_err();
    assert!(matches!(again, WriteError::Exists { .. }), "{again:?}");
}
