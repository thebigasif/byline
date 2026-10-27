//! The dry-run report of a Medium import, and the import written into a
//! site folder from end to end.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_content::{ContentTree, read_content};
use khalid_core::{Config, Environment};
use khalid_import::medium::{Plan, plan, read_export};
use khalid_routes::Routes;

/// A post file as Medium's export writes it: by Mara at `time` with the
/// address `canonical`, or a draft by nobody when `time` is empty.
#[cfg(test)]
fn post(title: &str, body: &str, time: &str, canonical: &str) -> String {
    let footer = if time.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p>By <a href="https://medium.com/@mara.lind" class="p-author h-card">Mara Lind</a> on <time class="dt-published" datetime="{time}">then</time>.</p><p><a href="{canonical}" class="p-canonical">Canonical link</a></p>"#
        )
    };
    format!(
        r#"<html><body><article class="h-entry"><header><h1 class="p-name">{title}</h1></header><section data-field="body" class="e-content"><section class="section"><div class="section-inner"><h3 class="graf graf--h3 graf--title">{title}</h3>{body}</div></section></section><footer>{footer}</footer></article></body></html>"#
    )
}

/// A hand-made export: a post on medium.com with a gist, a post from a
/// publication's own domain, a post from the new site's own host, a draft
/// and a file that is not a post, planned into a blog mounted at `/blog/`
/// on `northfield.example.com`.
#[cfg(test)]
fn planned() -> Plan {
    let text = "<p class=\"graf graf--p\">Rye needs time.</p>";
    let gist =
        "<figure class=\"graf graf--iframe\"><iframe src=\"/media/2b5f0c9e1d\"></iframe></figure>";
    let rye = post(
        "How to bake rye bread",
        &format!("{text}{gist}"),
        "2024-03-12T09:30:44.123Z",
        "https://medium.com/@mara.lind/how-to-bake-rye-bread-3f1a8c4e2b9d",
    );
    let spelt = post(
        "Spelt starter",
        text,
        "2024-04-01T07:00:00.000Z",
        "https://bakery.example.org/spelt-starter-0a1b2c3d4e5f",
    );
    let oats = post(
        "Oat porridge",
        text,
        "2024-05-01T06:00:00.000Z",
        "https://northfield.example.com/blog/oat-porridge-1c2d3e4f5a6b",
    );
    let draft = post("Spring flour", text, "", "");
    let export = read_export([
        ("2024-03-12_How-to-bake-rye-bread-3f1a8c4e2b9d.html", rye),
        ("2024-04-01_Spelt-starter-0a1b2c3d4e5f.html", spelt),
        ("2024-05-01_Oat-porridge-1c2d3e4f5a6b.html", oats),
        ("draft_Spring-flour-9e8d7c6b5a41.html", draft),
        ("notes.txt", "rye".to_owned()),
    ])
    .unwrap();
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    plan(&export, &site, &Routes::default())
}

/// `rest` inside the folder `dir`.
#[cfg(test)]
fn under(dir: &Path, rest: &str) -> PathBuf {
    let mut path = dir.to_owned();
    path.push(rest);
    path
}

#[test]
fn the_report_says_what_comes_in_what_is_mapped_and_what_is_not() {
    assert_eq!(
        planned().report().to_string(),
        "This is a dry run: nothing was written.\n\
         Coming in: 3 posts, 1 draft and 1 author.\n\
         Slugs kept: 3. Slugs made for drafts that had none: 1.\n\
         Old addresses mapped: 6, of which 6 move and 0 stay the same.\n\
         Old addresses on medium.com: 4. Medium cannot redirect them, so link each post there to its new home, or unpublish it.\n\
         Content with no place in Khalid, left out: frame (1 post).\n\
         Posts that could not come in: 0.\n\
         Files that are not Medium posts: notes.txt, it is not an HTML file.\n"
    );
}

#[test]
fn the_written_site_reads_back_with_every_post_and_author() {
    let site = tempfile::tempdir().unwrap();
    let plan = planned();
    let written = plan.write(site.path()).unwrap();
    assert_eq!(written.len(), plan.files.len());
    let content = read_content(&ContentTree::walk(site.path()).unwrap()).unwrap();
    let slug = |front: Option<&khalid_core::Slug>| front.map(ToString::to_string);
    let mut posts: Vec<Option<String>> = content
        .posts
        .iter()
        .map(|post| slug(post.front.slug.as_ref()))
        .collect();
    posts.sort();
    assert_eq!(
        posts,
        ["how-to-bake-rye-bread", "oat-porridge", "spelt-starter"].map(|s| Some(s.to_owned())),
        "no post is lost and every slug is kept"
    );
    let authors: Vec<Option<String>> = content
        .authors
        .iter()
        .map(|author| slug(author.front.slug.as_ref()))
        .collect();
    assert_eq!(authors, [Some("mara-lind".to_owned())]);
    assert!(under(site.path(), "drafts/en/spring-flour.md").is_file());
    let csv = fs::read_to_string(under(site.path(), "import/medium-redirects.csv")).unwrap();
    assert_eq!(csv.lines().count(), 7, "a header and six moves: {csv}");
    assert!(
        csv.contains("\n/blog/oat-porridge-1c2d3e4f5a6b,https://northfield.example.com/blog/oat-porridge/,301,exact\n"),
        "{csv}"
    );
}
