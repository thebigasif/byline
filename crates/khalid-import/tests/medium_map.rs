//! Where the old addresses of a Medium import lead, and its author files.

use std::path::Path;

use khalid_core::{Config, Environment};
use khalid_import::medium::{Plan, REDIRECTS_CSV, plan, read_export};
use khalid_import::{MapTarget, MoveStatus};
use khalid_routes::Routes;

/// A published post file as Medium's export writes it, by Mara, at the
/// address `canonical`.
#[cfg(test)]
fn post(title: &str, time: &str, canonical: &str) -> String {
    format!(
        r#"<html><body><article class="h-entry"><header><h1 class="p-name">{title}</h1></header><section data-field="body" class="e-content"><p>Rye needs time.</p></section><footer><p>By <a href="https://medium.com/@mara.lind" class="p-author h-card">Mara Lind</a> on <time class="dt-published" datetime="{time}">then</time>.</p><p><a href="{canonical}" class="p-canonical">Canonical link</a></p></footer></article></body></html>"#
    )
}

/// The plan of a post from medium.com, a post from a publication that
/// lived on the new site's own host, and a draft, into a blog mounted at
/// `/blog/` on `northfield.example.com`.
#[cfg(test)]
fn planned() -> Plan {
    let rye = post(
        "How to bake rye bread",
        "2024-03-12T09:30:44.123Z",
        "https://medium.com/@mara.lind/how-to-bake-rye-bread-3f1a8c4e2b9d",
    );
    let spelt = post(
        "Spelt starter",
        "2024-04-01T07:00:00.000Z",
        "https://northfield.example.com/blog/spelt-starter-0a1b2c3d4e5f",
    );
    let draft = "<html><body><article><section data-field=\"body\"><p>Soon.</p></section></article></body></html>";
    let export = read_export([
        (
            "2024-03-12_How-to-bake-rye-bread-3f1a8c4e2b9d.html",
            rye.as_str(),
        ),
        ("2024-04-01_Spelt-starter-0a1b2c3d4e5f.html", spelt.as_str()),
        ("draft_Soon-9e8d7c6b5a41.html", draft),
    ])
    .unwrap();
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    plan(&export, &site, &Routes::default())
}

#[test]
fn every_old_address_of_a_published_post_is_mapped_once() {
    let plan = planned();
    let map: Vec<(&str, &MapTarget, bool)> = plan
        .redirects
        .iter()
        .map(|entry| (entry.from.as_str(), &entry.to, entry.on_medium()))
        .collect();
    let to = |path: &str| MapTarget::Moved {
        to: format!("https://northfield.example.com{path}"),
        status: MoveStatus::Permanent,
    };
    let (rye, spelt) = (
        to("/blog/how-to-bake-rye-bread/"),
        to("/blog/spelt-starter/"),
    );
    assert_eq!(
        map,
        [
            (
                "https://medium.com/@mara.lind/how-to-bake-rye-bread-3f1a8c4e2b9d",
                &rye,
                true
            ),
            ("https://medium.com/p/3f1a8c4e2b9d", &rye, true),
            (
                "https://northfield.example.com/blog/spelt-starter-0a1b2c3d4e5f",
                &spelt,
                false
            ),
            ("https://medium.com/p/0a1b2c3d4e5f", &spelt, true),
        ],
        "a draft was never public, so it has no old address"
    );
}

#[test]
fn the_csv_lists_every_move_and_the_site_answers_its_own_old_paths() {
    let plan = planned();
    let csv = plan
        .files
        .iter()
        .find(|file| file.path == Path::new(REDIRECTS_CSV))
        .unwrap();
    assert_eq!(
        csv.text,
        "from,to,status,match\n\
         https://medium.com/@mara.lind/how-to-bake-rye-bread-3f1a8c4e2b9d,https://northfield.example.com/blog/how-to-bake-rye-bread/,301,exact\n\
         https://medium.com/p/3f1a8c4e2b9d,https://northfield.example.com/blog/how-to-bake-rye-bread/,301,exact\n\
         /blog/spelt-starter-0a1b2c3d4e5f,https://northfield.example.com/blog/spelt-starter/,301,exact\n\
         https://medium.com/p/0a1b2c3d4e5f,https://northfield.example.com/blog/spelt-starter/,301,exact\n"
    );
    let spelt = plan
        .files
        .iter()
        .find(|file| file.path == Path::new("content/en/posts/2024-04-01-spelt-starter.md"))
        .unwrap();
    assert!(
        spelt
            .text
            .contains("aliases:\n  - \"/blog/spelt-starter-0a1b2c3d4e5f\"\n"),
        "an old path inside the blog is answered by Khalid itself: {}",
        spelt.text
    );
    let rye = &plan.files[0].text;
    assert!(
        !rye.contains("aliases"),
        "medium.com is not this site: {rye}"
    );
}

#[test]
fn each_author_gets_one_file_and_their_posts_name_them() {
    let plan = planned();
    let authors: Vec<(&Path, &str)> = plan
        .files
        .iter()
        .filter(|file| file.path.starts_with("content/en/authors"))
        .map(|file| (file.path.as_path(), file.text.as_str()))
        .collect();
    assert_eq!(
        authors,
        [(
            Path::new("content/en/authors/mara-lind.md"),
            "---\nname: \"Mara Lind\"\nslug: \"mara-lind\"\nsame_as:\n  - \"https://medium.com/@mara.lind\"\n---\n\n"
        )],
        "the username mara.lind becomes a slug the site allows"
    );
    assert_eq!(plan.authors, 1);
    assert!(
        plan.files[0].text.contains("author: \"mara-lind\"\n"),
        "{}",
        plan.files[0].text
    );
}
