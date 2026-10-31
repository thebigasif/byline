//! `khalid build` fills in the reader features of plan section 4.12 from the
//! content and `site.toml`: bylines with roles, reviewers in the page and its
//! JSON-LD, reading time, related posts, the author box and the other
//! authors, each one off when `[reader]` says so.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// Copies the folder `from` into `into`, file by file.
#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, &entry.file_name().to_string_lossy());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A copy of the demo site where Asif reviewed the sitemap post, with
/// `reader` appended to `site.toml`, built into `out`.
#[cfg(test)]
fn build(reader: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let site = at(temp.path(), "site");
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    copy_dir(&demo, &site);
    let post = at(&site, "content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let text = fs::read_to_string(&post).unwrap();
    fs::write(
        &post,
        text.replacen(
            "\nauthor: lena-vogt\n",
            "\nauthor: lena-vogt\nreviewed_by: asif\n",
            1,
        ),
    )
    .unwrap();
    let settings = at(&site, "site.toml");
    let mut toml = fs::read_to_string(&settings).unwrap();
    toml.push_str(reader);
    fs::write(&settings, toml).unwrap();
    let out = at(temp.path(), "public");
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args([
            "--site",
            site.to_str().unwrap(),
            "build",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (temp, out)
}

#[cfg(test)]
fn page(out: &Path, rel: &str) -> String {
    fs::read_to_string(at(out, rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

#[test]
fn a_post_shows_its_people_reading_time_and_related_posts() {
    let (_temp, out) = build("");
    let post = page(&out, "what-a-sitemap-does/index.html");
    assert!(
        post.contains("<span class=\"person-role\">Technical SEO editor</span>"),
        "{post}"
    );
    assert!(
        post.contains(
            "<p class=\"reviewed-by\">Reviewed by <a href=\"/blog/authors/asif/\">Asif</a></p>"
        ),
        "{post}"
    );
    assert!(
        post.contains(
            "\"reviewedBy\":{\"@id\":\"https://www.example.com/blog/authors/asif/#person\"}"
        ),
        "{post}"
    );
    assert!(post.contains("min read</time>"), "{post}");
    assert!(post.contains("<section class=\"author-box\">"), "{post}");
    assert!(
        post.contains("She writes in English and German."),
        "the bio:\n{post}"
    );
    let related = post
        .split("<section class=\"related\">")
        .nth(1)
        .expect("related posts");
    // It links to the internal links post, and shares a category with the slow pages post.
    assert!(
        related.contains("/blog/why-internal-links-matter/"),
        "{related}"
    );
    assert!(related.contains("/blog/fixing-slow-pages/"), "{related}");
    assert!(
        !related.contains("/blog/what-a-sitemap-does/"),
        "never itself"
    );
    let home = page(&out, "index.html");
    assert!(
        home.contains("<span class=\"card-author\">Lena Vogt</span>"),
        "cards name their author:\n{home}"
    );
}

#[test]
fn a_post_with_three_sections_lists_them_and_a_shorter_one_does_not() {
    let (_temp, out) = build("");
    let long = page(&out, "what-a-sitemap-does/index.html");
    assert!(long.contains("<details class=\"toc-inline\">"), "{long}");
    assert!(long.contains("<nav class=\"toc-side\" aria-label=\"Contents\">"));
    let short = page(&out, "fixing-slow-pages/index.html");
    assert!(!short.contains("toc-inline"), "{short}");
}

#[test]
fn an_author_page_ends_with_the_other_authors() {
    let (_temp, out) = build("");
    let lena = page(&out, "authors/lena-vogt/index.html");
    let others = lena
        .split("<h2 id=\"other-authors\">")
        .nth(1)
        .expect("other authors");
    assert!(others.contains("/blog/authors/asif/"), "{others}");
    assert!(!others.contains("/blog/authors/lena-vogt/"), "{others}");
}

#[test]
fn features_switched_off_in_site_toml_are_gone() {
    let (_temp, out) = build(
        "\n[reader]\nrelated_posts = false\nauthor_box = false\nreviewed_by = false\nreading_time = false\ntable_of_contents = false\n",
    );
    let post = page(&out, "what-a-sitemap-does/index.html");
    for gone in [
        "class=\"related\"",
        "author-box",
        "Reviewed by",
        "min read",
        "toc-inline",
    ] {
        assert!(!post.contains(gone), "{gone} is still there:\n{post}");
    }
    assert!(post.contains("Technical SEO editor"), "the byline stays on");
}

/// The figure of the slow pages post, from `<figure>` to `</figure>`.
#[cfg(test)]
fn figure(out: &Path) -> String {
    let post = page(out, "fixing-slow-pages/index.html");
    let (_, body) = post.split_once("<figure>").expect("a figure in the body");
    let (inside, _) = body.split_once("</figure>").expect("its end");
    format!("<figure>{inside}</figure>")
}

#[test]
fn a_body_image_wider_than_the_column_links_its_whole_picture_to_its_widest_size() {
    let (_temp, out) = build("");
    let figure = figure(&out);
    let href = figure
        .split("<a class=\"enlarge\" href=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the picture links to its widest size");
    assert!(
        at(&out, href.trim_start_matches("/blog/")).is_file(),
        "{href}"
    );
    insta::assert_snapshot!(figure);
}

#[test]
fn click_to_enlarge_switched_off_leaves_the_picture_unlinked() {
    let (_temp, out) = build("\n[reader]\nenlarge_images = false\n");
    let figure = figure(&out);
    assert!(figure.contains("<picture>"), "{figure}");
    assert!(!figure.contains("enlarge"), "{figure}");
}

/// The linked mentions of the post at `rel`, if it shows any.
#[cfg(test)]
fn mentions(out: &Path, rel: &str) -> Option<String> {
    let post = page(out, rel);
    let (_, list) = post.split_once("<section class=\"mentions\">")?;
    let (list, _) = list.split_once("</section>")?;
    Some(format!("<section class=\"mentions\">{list}</section>"))
}

#[test]
fn a_post_lists_the_posts_that_link_to_it() {
    let (_temp, out) = build("");
    let list = mentions(&out, "why-internal-links-matter/index.html").expect("mentions");
    assert!(!list.contains("why-internal-links-matter/"), "never itself");
    insta::assert_snapshot!(list);
}

#[test]
fn linked_mentions_switched_off_are_gone() {
    let (_temp, out) = build("\n[reader]\nlinked_mentions = false\n");
    assert_eq!(mentions(&out, "why-internal-links-matter/index.html"), None);
}

/// The reader script's tag in the post at `rel`.
#[cfg(test)]
fn script_tag(out: &Path, rel: &str) -> String {
    let post = page(out, rel);
    let (_, tag) = post.split_once("<script src=").expect("the reader script");
    let (tag, _) = tag.split_once('>').unwrap();
    tag.to_owned()
}

#[test]
fn the_script_is_told_to_preview_links_and_links_stay_plain() {
    let (_temp, out) = build("");
    let tag = script_tag(&out, "what-a-sitemap-does/index.html");
    assert!(tag.ends_with(" data-previews"), "{tag}");
    let post = page(&out, "what-a-sitemap-does/index.html");
    assert!(
        post.contains("<a href=\"/blog/why-internal-links-matter/\">"),
        "the link itself is a plain link: {post}"
    );
}

#[test]
fn link_previews_switched_off_are_not_asked_for() {
    let (_temp, out) = build("\n[reader]\nlink_previews = false\n");
    let tag = script_tag(&out, "what-a-sitemap-does/index.html");
    assert!(!tag.contains("data-previews"), "{tag}");
}
