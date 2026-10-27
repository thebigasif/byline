//! Reading the `posts/` folder of a Medium export: each post's fields, and
//! the files that are not posts.

use khalid_import::medium::{ExportError, PostFileError, PostStatus, read_export};

/// A post file as Medium's export writes it, with `footer` inside the
/// article's footer.
#[cfg(test)]
fn post(title: &str, subtitle: &str, body: &str, footer: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta http-equiv="Content-Type" content="text/html; charset=utf-8"><title>{title}</title><style>p {{ margin: 0 }}</style></head><body><article class="h-entry">
<header>
<h1 class="p-name">{title}</h1>
</header>
<section data-field="subtitle" class="p-summary">
{subtitle}
</section>
<section data-field="body" class="e-content">
{body}
</section>
<footer>{footer}<p>Exported from <a href="https://medium.com">Medium</a> on March 1, 2025.</p></footer></article></body></html>"#
    )
}

const PUBLISHED: &str = r#"<p>By <a href="https://medium.com/@mara" class="p-author h-card">Mara Lind</a> on <a href="https://medium.com/p/3f1a8c4e2b9d"><time class="dt-published" datetime="2024-03-12T09:30:44.123Z">March 12, 2024</time></a>.</p><p><a href="https://medium.com/@mara/how-to-bake-rye-bread-3f1a8c4e2b9d" class="p-canonical">Canonical link</a></p>"#;

#[test]
fn reads_each_post_in_file_name_order_with_its_fields() {
    let rye = post(
        "How to bake rye bread",
        "A wetter dough &amp; a longer rest",
        "<p>Rye needs time.</p>",
        PUBLISHED,
    );
    let draft = post(
        "Spring flour",
        "",
        "<p>Not finished.</p>",
        r#"<p><a href="https://medium.com/p/9e8d7c6b5a41">View original.</a></p>"#,
    );
    let export = read_export([
        ("draft_Spring-flour-9e8d7c6b5a41.html", draft.as_str()),
        (
            "2024-03-12_How-to-bake-rye-bread-3f1a8c4e2b9d.html",
            rye.as_str(),
        ),
    ])
    .unwrap();
    assert!(export.unread.is_empty(), "{:?}", export.unread);
    let [rye, draft] = &export.posts[..] else {
        panic!("two posts: {:?}", export.posts);
    };
    assert_eq!(rye.id, "3f1a8c4e2b9d", "the file name sorts first");
    assert_eq!(
        rye.file,
        "2024-03-12_How-to-bake-rye-bread-3f1a8c4e2b9d.html"
    );
    assert_eq!(rye.title, "How to bake rye bread");
    assert_eq!(
        rye.subtitle.as_deref(),
        Some("A wetter dough & a longer rest")
    );
    assert_eq!(rye.body.trim(), "<p>Rye needs time.</p>");
    assert_eq!(rye.status, PostStatus::Published);
    assert_eq!(
        rye.published.map(|at| at.to_string()).as_deref(),
        Some("2024-03-12 09:30:44.123"),
        "times stay in UTC"
    );
    assert_eq!(
        rye.canonical.as_ref().map(ToString::to_string).as_deref(),
        Some("https://medium.com/@mara/how-to-bake-rye-bread-3f1a8c4e2b9d")
    );
    let author = rye.author.as_ref().unwrap();
    assert_eq!(author.name, "Mara Lind");
    assert_eq!(author.username(), Some("mara"));

    assert_eq!(draft.id, "9e8d7c6b5a41");
    assert_eq!(draft.status, PostStatus::Draft);
    assert_eq!((draft.published, &draft.canonical), (None, &None));
    assert_eq!(draft.subtitle, None, "an empty subtitle is none");
    assert_eq!(draft.author, None);
}

#[test]
fn a_post_without_a_headline_takes_its_title_from_the_page() {
    let text =
        post("Rye", "", "<p>Rye.</p>", PUBLISHED).replace("<h1 class=\"p-name\">Rye</h1>", "");
    let export = read_export([("2024-03-12_Rye-3f1a8c4e2b9d.html", text)]).unwrap();
    assert_eq!(export.posts[0].title, "Rye");
}

#[test]
fn files_that_are_not_posts_are_listed_with_why() {
    let rye = post("Rye", "", "<p>Rye.</p>", PUBLISHED);
    let late = rye.replace("2024-03-12T09:30:44.123Z", "last spring");
    let export = read_export([
        ("2024-03-12_Rye-3f1a8c4e2b9d.html", rye.as_str()),
        ("2024-03-13_Late-0a1b2c3d4e5f.html", late.as_str()),
        ("notes.txt", "rye"),
        ("profile.html", "<html><body><h3>Mara</h3></body></html>"),
    ])
    .unwrap();
    assert_eq!(export.posts.len(), 1);
    let unread: Vec<(&str, String)> = export
        .unread
        .iter()
        .map(|file| (file.file.as_str(), file.error.to_string()))
        .collect();
    assert_eq!(
        unread,
        [
            (
                "2024-03-13_Late-0a1b2c3d4e5f.html",
                "its publication time \"last spring\" is not a date".to_owned()
            ),
            ("notes.txt", "it is not an HTML file".to_owned()),
            (
                "profile.html",
                "it is not a Medium post: it has no body section".to_owned()
            ),
        ]
    );
    assert!(matches!(export.unread[1].error, PostFileError::NotHtml));
}

#[test]
fn a_folder_with_no_post_is_refused() {
    let none = read_export([("notes.txt", "rye")]).unwrap_err();
    assert!(
        matches!(none, ExportError::NoPosts { files: 1 }),
        "{none:?}"
    );
    assert_eq!(
        none.to_string(),
        "no file in the folder is a Medium post, so it is not the posts folder of a Medium export"
    );
    let empty = read_export(Vec::<(String, String)>::new()).unwrap_err();
    assert!(
        matches!(empty, ExportError::NoPosts { files: 0 }),
        "{empty:?}"
    );
}
