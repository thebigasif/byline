//! The reader features of plan section 4.12 as `khalid build` fills them in:
//! which ones are on, and the people a post names.

use std::collections::BTreeMap;

use khalid_content::SiteContent;
use khalid_core::{Config, Locale, Reader, Slug, UserMessage};
use khalid_render::{Feature, Features, Link, Person};

use crate::build_pages::{body_html, named, path_of};

/// The render crate's switches for `[reader]` in `site.toml`.
pub(crate) fn features(reader: &Reader) -> Features {
    [
        (reader.byline, Feature::Byline),
        (reader.reviewed_by, Feature::ReviewedBy),
        (reader.reading_time, Feature::ReadingTime),
        (reader.table_of_contents, Feature::TableOfContents),
        (reader.heading_links, Feature::HeadingLinks),
        (reader.enlarge_images, Feature::EnlargeImages),
        (reader.author_box, Feature::AuthorBox),
        (reader.related_posts, Feature::RelatedPosts),
        (reader.rich_cards, Feature::RichCards),
        (reader.topic_links, Feature::TopicLinks),
        (reader.authors_page, Feature::AuthorsPage),
        (reader.archive, Feature::Archive),
        (reader.search_shortcut, Feature::SearchShortcut),
    ]
    .into_iter()
    .filter(|&(on, _)| !on)
    .fold(Features::all(), |features, (_, off)| features.without(off))
}

/// Every author with a file in `locale`, by slug: name, page, role, bio and
/// profiles, as bylines, author boxes and author pages show them.
pub(crate) fn people(
    config: &Config,
    content: &SiteContent,
    locale: &Locale,
) -> Result<BTreeMap<Slug, Person>, UserMessage> {
    let mut people = BTreeMap::new();
    for doc in content.authors.iter().filter(|doc| &doc.locale == locale) {
        let front = &doc.front;
        let (name, slug) = named(config, doc, front.slug.as_ref(), front.name.as_deref())?;
        let person = Person {
            name,
            href: path_of(config, locale, &format!("authors/{slug}/")),
            role: front.role.clone(),
            image: None,
            bio: Some(body_html(config, doc)?),
            profiles: front
                .same_as
                .iter()
                .map(|url| Link {
                    name: url.host().to_owned(),
                    href: url.to_string(),
                })
                .collect(),
        };
        people.insert(slug, person);
    }
    Ok(people)
}

/// The author with `slug`, or, when no file in `locale` has that slug, a
/// person named by it, so a byline never drops a name. The linter reports
/// the missing file.
pub(crate) fn person(
    people: &BTreeMap<Slug, Person>,
    slug: &Slug,
    config: &Config,
    locale: &Locale,
) -> Person {
    people.get(slug).cloned().unwrap_or_else(|| Person {
        name: slug.to_string(),
        href: path_of(config, locale, &format!("authors/{slug}/")),
        role: None,
        image: None,
        bio: None,
        profiles: Vec::new(),
    })
}
