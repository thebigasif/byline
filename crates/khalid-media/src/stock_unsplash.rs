//! Unsplash's photo search, asked and read.

use serde::Deserialize;

use crate::stock::PER_PAGE;
use crate::{FoundPicture, PictureSource};

/// The width of the photo copy Unsplash calls `regular`, put in posts.
const REGULAR_WIDTH: u32 = 1080;

/// The marks Unsplash asks for on every link back to it.
const REFERRAL: &str = "utm_source=khalid&utm_medium=referral";

/// The query for page `page` of the photos for `words`, leaving out photos
/// unsuited to all ages.
pub(crate) fn query(words: &str, page: u32) -> Vec<(&'static str, String)> {
    vec![
        ("query", words.to_owned()),
        ("per_page", PER_PAGE.to_string()),
        ("page", page.to_string()),
        ("content_filter", "high".to_owned()),
    ]
}

/// The photos in an answer to [`query`].
pub(crate) fn read(body: &[u8]) -> Result<Vec<FoundPicture>, serde_json::Error> {
    let page: UnsplashPage = serde_json::from_slice(body)?;
    Ok(page.results.into_iter().map(photo).collect())
}

#[derive(Deserialize)]
struct UnsplashPage {
    results: Vec<UnsplashPhoto>,
}

#[derive(Deserialize)]
struct UnsplashPhoto {
    id: String,
    width: u32,
    height: u32,
    description: Option<String>,
    alt_description: Option<String>,
    urls: UnsplashUrls,
    links: PageLinks,
    user: UnsplashUser,
}

#[derive(Deserialize)]
struct UnsplashUrls {
    regular: String,
    small: String,
}

#[derive(Deserialize)]
struct PageLinks {
    html: String,
}

#[derive(Deserialize)]
struct UnsplashUser {
    name: String,
    links: PageLinks,
}

/// A found photo, with the `regular` copy, at most 1080 pixels wide, for
/// the post.
fn photo(found: UnsplashPhoto) -> FoundPicture {
    let width = found.width.clamp(1, REGULAR_WIDTH);
    let tall = u64::from(found.height) * u64::from(width);
    let height = (tall + u64::from(found.width) / 2)
        .checked_div(u64::from(found.width))
        .and_then(|height| u32::try_from(height).ok())
        .unwrap_or(1)
        .max(1);
    let description = found.alt_description.or(found.description);
    FoundPicture {
        source: PictureSource::Unsplash,
        id: found.id,
        description: description.unwrap_or_default().trim().to_owned(),
        preview: found.urls.small,
        address: found.urls.regular,
        width,
        height,
        credit: found.user.name,
        credit_link: referred(&found.user.links.html),
        page: referred(&found.links.html),
    }
}

/// `link` with Unsplash's referral marks.
fn referred(link: &str) -> String {
    let joiner = if link.contains('?') { '&' } else { '?' };
    format!("{link}{joiner}{REFERRAL}")
}
