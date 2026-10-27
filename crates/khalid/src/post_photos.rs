//! The images that follow a post or a person to other pages: each post's
//! lead image on its cards, and each author's photo beside their name in
//! bylines, cards, author boxes and author lists (plan section 4.12).

use std::collections::BTreeMap;

use khalid_render::{Loading, PageBody, Person, PostCard, RenderImage};

use crate::build_pages::{LanguagePages, Page};

/// How wide a photo beside a name is drawn: the `.avatar` size in the
/// stylesheet.
const AVATAR: &str = "2.75rem";

/// The images drawn at the top of post and author pages, by the page's
/// address, ready to show elsewhere.
#[derive(Debug, Clone, Default)]
pub(crate) struct Shown {
    /// Each post's lead image, for its cards.
    cards: BTreeMap<String, RenderImage>,
    /// Each author's photo, for the places their name appears.
    photos: BTreeMap<String, RenderImage>,
}

impl Shown {
    /// Keeps `image`, drawn at the top of `page`, for the other pages that
    /// show the post or person. Elsewhere it loads when it nears the
    /// screen, with no caption.
    pub(crate) fn add(&mut self, page: &Page, mut image: RenderImage) {
        image.loading = Loading::Lazy;
        image.caption = None;
        let address = page.meta.path.clone();
        match &page.body {
            PageBody::Post(_) => {
                self.cards.insert(address, image);
            }
            PageBody::Author(_) => {
                AVATAR.clone_into(&mut image.sizes);
                self.photos.insert(address, image);
            }
            _ => {}
        }
    }
}

/// Puts the images `shown` holds on every card and beside every name on
/// the pages of `languages`, except the home page: `khalid serve` answers
/// with it before any image has its sizes, and it must be the page the
/// full render makes.
pub(crate) fn attach(languages: &mut [LanguagePages], shown: &Shown) {
    if shown.cards.is_empty() && shown.photos.is_empty() {
        return;
    }
    let pages = languages
        .iter_mut()
        .flat_map(|language| &mut language.pages);
    for page in pages {
        match &mut page.body {
            PageBody::Post(post) => {
                post.authors
                    .iter_mut()
                    .for_each(|person| photo(person, shown));
                cards(&mut post.related, shown);
            }
            PageBody::Author(author) => {
                author
                    .others
                    .iter_mut()
                    .for_each(|person| photo(person, shown));
                cards(&mut author.posts, shown);
            }
            PageBody::Authors(authors) => {
                let people = authors.authors.iter_mut().map(|entry| &mut entry.person);
                people.for_each(|person| photo(person, shown));
            }
            PageBody::Collection(listing)
            | PageBody::Tag(listing)
            | PageBody::Category(listing)
            | PageBody::Archive(listing) => cards(&mut listing.posts, shown),
            _ => {}
        }
    }
}

/// Gives each of `posts` its post's lead image and its author's photo.
fn cards(posts: &mut [PostCard], shown: &Shown) {
    for card in posts {
        if let Some(image) = shown.cards.get(&card.href) {
            card.image = Some(image.clone());
        }
        if let Some(author) = &mut card.author {
            photo(author, shown);
        }
    }
}

/// Gives `person` the photo from their author page, if it has one.
fn photo(person: &mut Person, shown: &Shown) {
    if let Some(image) = shown.photos.get(&person.href) {
        person.image = Some(image.clone());
    }
}
