//! The reader features of plan section 4.12 as `khalid build` fills them in:
//! which ones are on, the people a post names, its contents and the posts
//! related to it.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use khalid_content::{Heading, PostFront, Rendered, SiteContent, Wikilinks};
use khalid_core::{AbsoluteUrl, Config, Locale, Reader, Slug, UserMessage};
use khalid_render::{
    Contents, ContentsHeading, Feature, Features, HeadingLevel, Link, Person, PostTopics,
};
use khalid_seo::AuthorProfile;

use crate::build_message;
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
        (reader.linked_mentions, Feature::LinkedMentions),
        (reader.link_previews, Feature::LinkPreviews),
    ]
    .into_iter()
    .filter(|&(on, _)| !on)
    .fold(Features::all(), |features, (_, off)| features.without(off))
}

/// Every author with a file in `locale`, by slug: name, page, role, bio and
/// profiles, as bylines, author boxes and author pages show them.
pub(crate) fn people(
    config: &Config,
    links: &Wikilinks,
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
            bio: Some(body_html(config, links, doc)?),
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

/// The reviewers `front` names that have an author file: links for the
/// byline, and profiles for `reviewedBy`. A slug with no file is left out,
/// so no page links to a missing author; the linter names it.
///
/// # Errors
///
/// A [`UserMessage`] when the site address cannot form a reviewer's address.
pub(crate) fn reviewers(
    front: &PostFront,
    people: &BTreeMap<Slug, Person>,
    config: &Config,
) -> Result<(Vec<Link>, Vec<AuthorProfile>), UserMessage> {
    let mut links = Vec::new();
    let mut profiles = Vec::new();
    for person in front.reviewed_by.iter().filter_map(|slug| people.get(slug)) {
        let address = format!(
            "{}{}",
            config.address().as_str().trim_end_matches('/'),
            person.href
        );
        let mut profile = AuthorProfile::new(&person.name);
        profile.url =
            Some(AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))?);
        profile.job_title.clone_from(&person.role);
        profiles.push(profile);
        links.push(Link {
            name: person.name.clone(),
            href: person.href.clone(),
        });
    }
    Ok((links, profiles))
}

/// The table of contents of a body with `headings`: its H2s and H3s.
pub(crate) fn contents(headings: &[Heading]) -> Option<Contents> {
    let listed = headings
        .iter()
        .filter_map(|heading| {
            let level = match heading.level {
                2 => HeadingLevel::Two,
                3 => HeadingLevel::Three,
                _ => return None,
            };
            Some(ContentsHeading {
                level,
                id: heading.id.clone(),
                text: heading.text.clone(),
            })
        })
        .collect();
    Contents::of(listed)
}

/// What the related-posts choice reads of the post with `slug`: its date,
/// category and the posts its body links to, found by their paths in
/// `slugs`.
pub(crate) fn topics(
    slug: &Slug,
    date: NaiveDate,
    front: &PostFront,
    rendered: &Rendered,
    slugs: &BTreeMap<String, String>,
) -> PostTopics {
    PostTopics {
        slug: slug.to_string(),
        date,
        category: front.category.as_ref().map(Slug::to_string),
        tags: Vec::new(),
        links: rendered
            .links
            .iter()
            .filter(|link| link.internal)
            .filter_map(|link| slugs.get(link.href.split('#').next().unwrap_or_default()))
            .cloned()
            .collect(),
    }
}
