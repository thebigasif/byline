//! GIPHY's GIF search, asked and read.

use khalid_core::Secret;
use serde::Deserialize;

use crate::stock::PER_PAGE;
use crate::{FoundPicture, PictureSource};

/// The query for page `page` of the GIFs for `words`, rated G only. GIPHY
/// takes its `key` in the query, so the address is never shown.
pub(crate) fn query(key: &Secret<String>, words: &str, page: u32) -> Vec<(&'static str, String)> {
    let offset = page.saturating_sub(1).saturating_mul(PER_PAGE);
    vec![
        ("api_key", key.expose().clone()),
        ("q", words.to_owned()),
        ("limit", PER_PAGE.to_string()),
        ("offset", offset.to_string()),
        ("rating", "g".to_owned()),
    ]
}

/// The GIFs in an answer to [`query`].
pub(crate) fn read(body: &[u8]) -> Result<Vec<FoundPicture>, serde_json::Error> {
    let page: GiphyPage = serde_json::from_slice(body)?;
    Ok(page.data.into_iter().map(gif).collect())
}

#[derive(Deserialize)]
struct GiphyPage {
    data: Vec<Gif>,
}

#[derive(Deserialize)]
struct Gif {
    id: String,
    #[serde(default)]
    title: String,
    url: String,
    images: GifImages,
    user: Option<GiphyUser>,
}

#[derive(Deserialize)]
struct GifImages {
    fixed_width: GifImage,
    downsized_medium: Option<GifImage>,
    original: GifImage,
}

/// GIPHY writes sizes as text, such as `"480"`.
#[derive(Deserialize)]
struct GifImage {
    url: String,
    width: String,
    height: String,
}

#[derive(Deserialize)]
struct GiphyUser {
    display_name: String,
    profile_url: String,
}

/// A found GIF, with GIPHY's copy of at most 5 MB for the post.
fn gif(found: Gif) -> FoundPicture {
    let images = found.images;
    let shown = images.downsized_medium.unwrap_or(images.original);
    let size = |text: &str| text.parse::<u32>().unwrap_or(0);
    let (credit, credit_link) = match found.user {
        Some(user) if !user.display_name.is_empty() => (user.display_name, user.profile_url),
        _ => ("GIPHY".to_owned(), found.url.clone()),
    };
    FoundPicture {
        source: PictureSource::Giphy,
        id: found.id,
        description: found.title.trim().to_owned(),
        preview: images.fixed_width.url,
        width: size(&shown.width),
        height: size(&shown.height),
        address: shown.url,
        credit,
        credit_link,
        page: found.url,
    }
}
