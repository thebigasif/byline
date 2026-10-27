//! The files a site gets once per import: one per author, and the
//! categories with their names.

use std::fmt::Write as _;
use std::path::PathBuf;

use khalid_core::Config;

use crate::front_matter::{FrontMatter, quoted};
use crate::wordpress::names::{author_slug, category_slug, decoded};
use crate::wordpress::{Export, PlannedFile};

/// One file per author under `content/<lang>/authors/`, named by the same
/// slug their posts use. Two authors whose names make one slug share the
/// first one's file.
pub(crate) fn author_files(export: &Export, site: &Config) -> Vec<PlannedFile> {
    let locale = site.default_language();
    let mut files: Vec<PlannedFile> = Vec::new();
    for author in &export.authors {
        let Some(slug) = author_slug(author, site) else {
            continue;
        };
        let path = PathBuf::from(format!("content/{locale}/authors/{slug}.md"));
        if files.iter().any(|file| file.path == path) {
            continue;
        }
        let mut front = FrontMatter::default();
        front
            .text("name", &decoded(&author.display_name))
            .text("slug", slug.as_str());
        files.push(PlannedFile {
            path,
            text: front.file(""),
        });
    }
    files
}

/// `categories.toml`, with each category's slug and its name in the site's
/// language, and how many it lists, or none when the export has none.
pub(crate) fn categories_file(export: &Export, site: &Config) -> Option<(PlannedFile, usize)> {
    let locale = site.default_language();
    let mut text = format!(
        "# Categories from the WordPress export of {}.\n",
        decoded(&export.site.title).replace('\n', " ")
    );
    let mut slugs = Vec::new();
    for term in &export.categories {
        let Some(slug) = category_slug(&term.slug, export, site) else {
            continue;
        };
        if slugs.contains(&slug) {
            continue;
        }
        let name = quoted(&decoded(&term.name));
        // Writing to a String cannot fail.
        let _ = write!(
            text,
            "\n[[categories]]\nslug = \"{slug}\"\n{locale} = {name}\n"
        );
        slugs.push(slug);
    }
    let file = PlannedFile {
        path: PathBuf::from("categories.toml"),
        text,
    };
    (!slugs.is_empty()).then_some((file, slugs.len()))
}
