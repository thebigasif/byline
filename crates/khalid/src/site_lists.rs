//! The Content API's site lists on a running site: authors, tags and
//! categories from the index in `data/site.db`, collections from the content,
//! and settings and menus from `site.toml`.

use std::path::Path;

use khalid_api::content_store::SiteResources;
use khalid_api::resources::{Author, ContentCollection, Menu, Settings, Term};
use khalid_content::SiteContent;
use khalid_core::{AbsoluteUrl, Config, Locale, Slug, UserMessage};
use khalid_db::{Database, IndexedTerm, SiteId};

use crate::{build_message, build_pages, db_message, index};

/// The menus every page has. Nothing fills them yet, so they are empty, as
/// on the pages themselves.
const MENUS: [&str; 2] = ["primary", "footer"];

/// The site's lists, read from the index of `site` in `db` after a rebuild
/// of the `folder`, with each author's role, introduction and profiles from
/// `content`.
///
/// # Errors
///
/// Returns what, why and fix when the index cannot be read or an address
/// cannot be made from the site's settings.
pub(crate) async fn of(
    db: &Database,
    site: SiteId,
    folder: &Path,
    config: &Config,
    content: &SiteContent,
) -> Result<SiteResources, UserMessage> {
    let unusable = |error| db_message::of(folder, &error);
    let mut authors = Vec::new();
    for row in db.authors(site).await.map_err(unusable)? {
        let url = address(config, &row.locale, &format!("authors/{}/", row.slug))?;
        let mut author = Author::new(row.slug, &row.name, row.locale).with_url(url);
        let doc = content
            .authors
            .iter()
            .find(|doc| index::slashed(&doc.path).as_deref() == Some(row.path.as_str()));
        if let Some(doc) = doc {
            if let Some(role) = &doc.front.role {
                author = author.with_role(role);
            }
            let bio = doc.body.trim();
            if !bio.is_empty() {
                author = author.with_bio(bio);
            }
            author = author.with_same_as(doc.front.same_as.clone());
        }
        authors.push(author);
    }
    let terms = |rows: Vec<IndexedTerm>| -> Vec<Term> {
        let term = |row: IndexedTerm| Term::new(row.slug, &row.name, row.locale);
        rows.into_iter().map(term).collect()
    };
    let tags = terms(db.tags(site).await.map_err(unusable)?);
    let categories = terms(db.categories(site).await.map_err(unusable)?);
    let home = address(config, config.default_language(), "")?;
    let settings = Settings::new(
        config.name(),
        home,
        config.default_language().clone(),
        config.languages().to_vec(),
    );
    Ok(SiteResources::default()
        .with_authors(authors)
        .with_tags(tags)
        .with_categories(categories)
        .with_collections(collections(config, content))
        .with_menus(menus(config))
        .with_settings(settings))
}

/// Every collection folder of each language that holds entries, posts
/// first, then pages, glossary, case studies and changelog, with the title
/// and description of its `_index.md`.
fn collections(config: &Config, content: &SiteContent) -> Vec<ContentCollection> {
    let folders = content
        .posts
        .iter()
        .map(|doc| (&doc.locale, &doc.collection))
        .chain(
            content
                .pages
                .iter()
                .map(|doc| (&doc.locale, &doc.collection)),
        )
        .chain(
            content
                .glossary
                .iter()
                .map(|doc| (&doc.locale, &doc.collection)),
        )
        .chain(
            content
                .case_studies
                .iter()
                .map(|doc| (&doc.locale, &doc.collection)),
        )
        .chain(
            content
                .changelog
                .iter()
                .map(|doc| (&doc.locale, &doc.collection)),
        );
    let folders: Vec<(&Locale, &String)> = folders.collect();
    let mut found = Vec::new();
    for locale in config.languages() {
        let mut seen: Vec<&str> = Vec::new();
        for (_, folder) in folders.iter().filter(|(of, _)| *of == locale) {
            let Ok(slug) = Slug::try_new(folder) else {
                continue;
            };
            if seen.contains(&folder.as_str()) {
                continue;
            }
            seen.push(folder);
            let mut collection = ContentCollection::new(slug, locale.clone());
            let intro = content
                .indexes
                .iter()
                .find(|doc| &doc.locale == locale && &doc.collection == *folder);
            if let Some(front) = intro.map(|doc| &doc.front) {
                if let Some(title) = &front.title {
                    collection = collection.with_title(title);
                }
                if let Some(description) = &front.description {
                    collection = collection.with_description(description);
                }
            }
            found.push(collection);
        }
    }
    found
}

/// The header and footer menu of each language, with no links.
fn menus(config: &Config) -> Vec<Menu> {
    let mut menus = Vec::new();
    for locale in config.languages() {
        for name in MENUS {
            if let Ok(slug) = Slug::try_new(name) {
                menus.push(Menu::new(slug, locale.clone()));
            }
        }
    }
    menus
}

/// The full address of `rest` in `locale`, such as the German author page.
fn address(config: &Config, locale: &Locale, rest: &str) -> Result<AbsoluteUrl, UserMessage> {
    let path = build_pages::path_of(config, locale, rest);
    let address = format!("{}{path}", config.address().as_str().trim_end_matches('/'));
    AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))
}
