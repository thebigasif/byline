//! The pages `khalid serve` renders, checked on the demo site.

use std::fs;
use std::path::{Path, PathBuf};

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use khalid_core::{Config, Environment, FixedClock, SystemClock};
use khalid_routes::Routes;

use super::{Assets, ContentFeed, ContentTree, Render, SitePages, read_content};

fn fixture(name: &str) -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    path.push("../../fixtures");
    path.push(name);
    path
}

fn pages(site: &Path) -> SitePages {
    let config = Config::load(site, &Environment::empty()).unwrap();
    let routes = Routes::load(site).unwrap();
    SitePages::new(site, config, routes, None, Arc::new(SystemClock)).unwrap()
}

#[test]
fn a_saved_broken_wikilink_stops_the_render_and_names_its_file_and_line() {
    let site = fixture("invalid/content-broken-wikilink");
    let message = pages(&site).render_all().unwrap_err();
    let what = message.what();
    assert!(
        what.ends_with("why-links-matter.md, line 10: [[link-equity]] matches no post, page or glossary term in en."),
        "{what}"
    );
}

#[test]
fn the_first_pages_are_the_full_renders_home_pages_and_stylesheet() {
    let site = demo_copy();
    let content = read_content(&ContentTree::walk(site.path()).unwrap()).unwrap();
    let source = pages(site.path());
    let first = source.render_first(&content).unwrap();
    let all = source.render_all().unwrap().pages;
    let paths: Vec<&str> = first.iter().map(|page| page.path().as_str()).collect();
    assert_eq!(
        paths.len(),
        4,
        "the stylesheet, the script, two homes: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|path| path.starts_with("/blog/assets/reader."))
    );
    assert!(paths.contains(&"/blog/"), "{paths:?}");
    assert!(paths.contains(&"/blog/de/"), "{paths:?}");
    for page in &first {
        assert!(all.contains(page), "{} differs", page.path().as_str());
    }
}

#[test]
fn served_pages_link_each_wikilink() {
    let pages = pages(&fixture("demo-site")).render_all().unwrap().pages;
    let link = "<a href=\"/blog/glossary/topical-authority/\">Topical authority</a>";
    // The sitemap page links every page, so only bodies count.
    let linking = pages
        .iter()
        .filter(|page| page.path().as_str() != "/blog/sitemap/")
        .filter(|page| String::from_utf8_lossy(&page.body().unwrap()).contains(link));
    assert_eq!(linking.count(), 2);
}

#[test]
fn the_checked_content_renders_the_same_pages_as_the_folder() {
    let demo = fixture("demo-site");
    let tree = ContentTree::walk(&demo).unwrap();
    let content = read_content(&tree).unwrap();
    let assets = Assets::of(&tree);
    assert_ne!(assets, Assets::default(), "the demo has a folder post");
    drop(tree);
    let source = pages(&demo);
    let given = source.render_content(&content, assets).unwrap();
    let read = source.render_all().unwrap();
    assert!(given.pages.len() > 10, "{} pages", given.pages.len());
    assert_eq!(given.pages, read.pages);
    assert_eq!(given.not_found, read.not_found);
    assert_eq!(given.moves, read.moves);
}

#[test]
fn the_start_render_finishing_after_a_saved_one_hands_on_nothing() {
    let site = demo_copy();
    let checked = read_content(&ContentTree::walk(site.path()).unwrap()).unwrap();
    let config = Config::load(site.path(), &Environment::empty()).unwrap();
    let routes = Routes::load(site.path()).unwrap();
    let (feed, mut catalogs) = ContentFeed::for_tests();
    let clock = Arc::new(SystemClock);
    let source = SitePages::new(site.path(), config, routes, Some(feed), clock).unwrap();
    // A post saved while the start render of the checked content runs.
    let mut file = site.path().to_path_buf();
    file.push("content/en/posts/2026-10-30-a-saved-post.md");
    let text = "---\ntitle: A saved post\nslug: a-saved-post\ndate: 2026-10-30\nauthor: lena-vogt\ncategory: technical-seo\n---\n\nSaved while serve started, beside [what a sitemap does](/blog/what-a-sitemap-does/).\n";
    fs::write(&file, text).unwrap();
    source.render_all().unwrap();
    let saved = catalogs
        .try_recv()
        .expect("the saved render hands on its catalog");
    let posts = outlined(&source);
    assert_eq!(saved.index.posts.len(), checked.posts.len() + 1);

    // The start render, of content read before the save, finishes last.
    source.render_content(&checked, Assets::default()).unwrap();
    assert!(
        catalogs.try_recv().is_err(),
        "older content would undo the save"
    );
    assert_eq!(outlined(&source), posts);
}

fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// The paths of the posts the last full render of `source` read.
fn outlined(source: &SitePages) -> Vec<PathBuf> {
    let outline = source.0.outline.lock().unwrap();
    outline
        .content
        .posts
        .iter()
        .map(|doc| doc.path.clone())
        .collect()
}

/// The demo site in a folder of its own, which a test may edit.
fn demo_copy() -> tempfile::TempDir {
    let site = tempfile::tempdir().unwrap();
    copy_dir(&fixture("demo-site"), site.path());
    site
}

/// Whether the file at `path` in `site` holds a wikilink.
fn links_a_term(site: &Path, path: &Path) -> bool {
    let mut file = site.to_path_buf();
    file.push(path);
    fs::read_to_string(file).unwrap().contains("[[")
}

#[test]
fn each_saved_post_alone_renders_the_page_the_full_render_makes() {
    let site = demo_copy();
    let source = pages(site.path());
    let unread = vec![PathBuf::from(
        "content/en/posts/2026-09-28-what-a-sitemap-does.md",
    )];
    assert_eq!(
        source.render_saved(&unread).unwrap(),
        None,
        "no full render yet"
    );

    let all = source.render_all().unwrap().pages;
    let (linking, posts): (Vec<PathBuf>, Vec<PathBuf>) = outlined(&source)
        .into_iter()
        .partition(|path| links_a_term(site.path(), path));
    assert_eq!(posts.len(), 6, "{posts:?}");
    for post in &posts {
        let saved = source.render_saved(std::slice::from_ref(post)).unwrap();
        let saved = saved.unwrap_or_else(|| panic!("{} alone", post.display()));
        assert_eq!(saved.len(), 2, "the post and its translation");
        for page in &saved {
            assert!(all.contains(page), "{} differs", page.path().as_str());
        }
    }
    assert_eq!(linking.len(), 4, "{linking:?}");
    assert_eq!(source.render_saved(&linking).unwrap(), None, "a wikilink");
    let author = vec![PathBuf::from("content/en/authors/asif.md")];
    assert_eq!(source.render_saved(&author).unwrap(), None);
}

#[test]
fn a_saved_edit_that_changes_the_related_posts_renders_the_next_full_render_page() {
    let site = demo_copy();
    let source = pages(site.path());
    source.render_all().unwrap();
    let post = PathBuf::from("content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let mut file = site.path().to_path_buf();
    file.push(&post);
    let text = fs::read_to_string(&file).unwrap();
    let edited = text.replace(
        "title: What a sitemap does,",
        "title: What a sitemap is for,",
    );
    let link = "\nSee [the topic cluster plan](/blog/planning-a-topic-cluster/).\n";
    fs::write(&file, format!("{edited}{link}")).unwrap();

    let saved = source.render_saved(&[post]).unwrap().unwrap();
    let all = source.render_all().unwrap().pages;
    let page = saved
        .iter()
        .find(|page| page.path().as_str() == "/blog/what-a-sitemap-does/")
        .unwrap();
    let body = page.body().unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("What a sitemap is for"), "the edit shows");
    assert!(all.contains(page), "the saved page is the full render's");
}

#[test]
fn a_served_feed_with_no_posts_is_dated_by_the_given_clock() {
    let site = demo_copy();
    // The German glossary links German posts, so it goes with them.
    for folder in ["content/de/posts", "content/de/glossary"] {
        let mut path = site.path().to_path_buf();
        path.push(folder);
        fs::remove_dir_all(path).unwrap();
    }
    let config = Config::load(site.path(), &Environment::empty()).unwrap();
    let routes = Routes::load(site.path()).unwrap();
    let launch = Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap();
    let clock = Arc::new(FixedClock::new(launch));
    let source = SitePages::new(site.path(), config, routes, None, clock).unwrap();
    let pages = source.render_all().unwrap().pages;
    let feed = pages
        .iter()
        .find(|page| page.path().as_str() == "/blog/de/feed.xml")
        .unwrap();
    let rss = String::from_utf8_lossy(&feed.body().unwrap()).into_owned();
    assert!(
        rss.contains("<lastBuildDate>Mon, 26 Oct 2026 09:00:00 +0000</lastBuildDate>"),
        "{rss}"
    );
}
