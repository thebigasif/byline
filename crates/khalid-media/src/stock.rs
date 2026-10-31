//! Free photos from Unsplash and GIFs from GIPHY, found for the editor
//! (decisions E6 to E8 in `docs/decisions/2026-10-31-editor-emoji-photos-and-edits.md`).
//!
//! A chosen picture stays where it came from, as Unsplash's API terms ask,
//! so this module only searches and tells Unsplash when a photo is used.

use khalid_core::{Environment, Secret};
use khalid_outbound::Outbound;

use crate::{stock_giphy, stock_unsplash};

/// Unsplash's API, where photo search asks.
const UNSPLASH_API: &str = "https://api.unsplash.com/";

/// GIPHY's API, where GIF search asks.
const GIPHY_API: &str = "https://api.giphy.com/";

/// How many pictures one page of results holds.
pub(crate) const PER_PAGE: u32 = 24;

/// The longest search, in characters.
const MAX_WORDS: usize = 100;

/// Where a picture comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureSource {
    /// Free photos from unsplash.com.
    Unsplash,
    /// GIFs from giphy.com.
    Giphy,
}

impl PictureSource {
    /// The environment variable that holds the source's key.
    pub fn variable(self) -> &'static str {
        match self {
            Self::Unsplash => Environment::UNSPLASH_ACCESS_KEY,
            Self::Giphy => Environment::GIPHY_API_KEY,
        }
    }
}

/// Whether a source may be asked, and with which key.
#[derive(Debug)]
pub enum Access {
    /// The owner has not turned it on in `site.toml`.
    Off,
    /// It is on, but its key is not set in the environment.
    NoKey,
    /// It is on, with this key.
    Key(Secret<String>),
}

impl Access {
    /// A source the owner turned on, with the key the environment holds for
    /// it, if any.
    pub fn on_with(key: Option<Secret<String>>) -> Self {
        key.map_or(Self::NoKey, Self::Key)
    }
}

/// One photo or GIF a search found.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FoundPicture {
    /// Where it comes from.
    pub source: PictureSource,
    /// Its identity at the source.
    pub id: String,
    /// Words its source has for it, a start for alt text; may be empty.
    pub description: String,
    /// A small copy for the list of results.
    pub preview: String,
    /// The address to put in a post.
    pub address: String,
    /// The width of the picture at `address`, in pixels.
    pub width: u32,
    /// The height of the picture at `address`, in pixels.
    pub height: u32,
    /// Who made it, or `GIPHY` for a GIF with no maker named.
    pub credit: String,
    /// The maker's page, with the referral marks Unsplash asks for.
    pub credit_link: String,
    /// The picture's own page at its source, with the same marks.
    pub page: String,
}

/// Why a search found nothing, or a use was not reported.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StockError {
    /// The owner has not turned the source on.
    #[error("this search is turned off in site.toml")]
    Off,
    /// The source is on, but its key is not set.
    #[error("no key is set in {variable}")]
    NoKey {
        /// The environment variable to set.
        variable: &'static str,
    },
    /// The source refused the key.
    #[error("the source refused the key")]
    KeyRefused,
    /// The source asks to wait, because the key made too many requests.
    #[error("the source asks to wait before the next search")]
    Busy,
    /// The source has no picture with this identity.
    #[error("the source has no picture with this identity")]
    NotFound,
    /// The source could not be reached or answered with a failure. The
    /// reason never holds the address, which may carry the key.
    #[error("the source could not be reached: {reason}")]
    Unreachable {
        /// What went wrong.
        reason: String,
    },
    /// The source's answer could not be read.
    #[error("the source's answer could not be read")]
    Unreadable,
}

/// The base addresses of the APIs, each ending in `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockApis {
    /// Unsplash's API.
    pub unsplash: String,
    /// GIPHY's API.
    pub giphy: String,
}

impl Default for StockApis {
    /// The real APIs.
    fn default() -> Self {
        Self {
            unsplash: UNSPLASH_API.to_owned(),
            giphy: GIPHY_API.to_owned(),
        }
    }
}

/// Photo and GIF search, each asked only when the owner turned it on.
#[derive(Debug)]
pub struct StockSearch {
    outbound: Outbound,
    unsplash: Access,
    giphy: Access,
    apis: StockApis,
}

impl StockSearch {
    /// Searches through `outbound`, with Unsplash as `unsplash` allows.
    /// GIF search is off until [`StockSearch::with_gifs`].
    pub fn new(outbound: Outbound, unsplash: Access) -> Self {
        Self {
            outbound,
            unsplash,
            giphy: Access::Off,
            apis: StockApis::default(),
        }
    }

    /// The same search with GIPHY as `giphy` allows.
    #[must_use]
    pub fn with_gifs(self, giphy: Access) -> Self {
        Self { giphy, ..self }
    }

    /// The same search asking `apis` instead of the real ones, for tests.
    #[must_use]
    pub fn at(self, apis: StockApis) -> Self {
        Self { apis, ..self }
    }

    /// Whether the owner turned `source` on, with or without its key.
    pub fn is_on(&self, source: PictureSource) -> bool {
        !matches!(self.access(source), Access::Off)
    }

    /// Page `page` of the pictures `source` has for `words`; the first page
    /// is 1. Unsplash leaves out photos unsuited to all ages, and GIPHY
    /// gives GIFs rated G only.
    ///
    /// # Errors
    ///
    /// Returns [`StockError::Off`] or [`StockError::NoKey`] before asking,
    /// [`StockError::KeyRefused`] or [`StockError::Busy`] as the source
    /// answers, [`StockError::Unreachable`] when it cannot be reached, and
    /// [`StockError::Unreadable`] for an answer that is not its JSON.
    pub async fn search(
        &self,
        source: PictureSource,
        words: &str,
        page: u32,
    ) -> Result<Vec<FoundPicture>, StockError> {
        let key = self.key(source)?;
        let words: String = words.trim().chars().take(MAX_WORDS).collect();
        let page = page.max(1);
        let found = match source {
            PictureSource::Unsplash => {
                let query = encode(&stock_unsplash::query(&words, page));
                let url = format!("{}search/photos?{query}", self.apis.unsplash);
                stock_unsplash::read(&self.ask(&url, Some(key)).await?)
            }
            PictureSource::Giphy => {
                let query = encode(&stock_giphy::query(key, &words, page));
                let url = format!("{}v1/gifs/search?{query}", self.apis.giphy);
                stock_giphy::read(&self.ask(&url, None).await?)
            }
        };
        found.map_err(|_| StockError::Unreadable)
    }

    /// Tells Unsplash the photo `id` was put in a post, as its API terms
    /// ask whenever a photo is used.
    ///
    /// # Errors
    ///
    /// Returns [`StockError::NotFound`] for an identity Unsplash never
    /// makes or does not know, and otherwise the errors of
    /// [`StockSearch::search`].
    pub async fn used(&self, id: &str) -> Result<(), StockError> {
        let key = self.key(PictureSource::Unsplash)?;
        let fits = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if id.is_empty() || id.len() > 64 || !id.chars().all(fits) {
            return Err(StockError::NotFound);
        }
        let url = format!("{}photos/{id}/download", self.apis.unsplash);
        self.ask(&url, Some(key)).await.map(drop)
    }

    fn access(&self, source: PictureSource) -> &Access {
        match source {
            PictureSource::Unsplash => &self.unsplash,
            PictureSource::Giphy => &self.giphy,
        }
    }

    fn key(&self, source: PictureSource) -> Result<&Secret<String>, StockError> {
        match self.access(source) {
            Access::Off => Err(StockError::Off),
            Access::NoKey => Err(StockError::NoKey {
                variable: source.variable(),
            }),
            Access::Key(key) => Ok(key),
        }
    }

    /// The body `url` answers with, sending the Unsplash `key` as its
    /// header when there is one.
    async fn ask(&self, url: &str, key: Option<&Secret<String>>) -> Result<Vec<u8>, StockError> {
        let mut request = http::Request::get(url).header("Accept-Version", "v1");
        if let Some(key) = key {
            request = request.header("Authorization", format!("Client-ID {}", key.expose()));
        }
        let request = request
            .body(Vec::new())
            .map_err(|_| StockError::KeyRefused)?;
        let unreachable = |error: khalid_outbound::OutboundError| StockError::Unreachable {
            reason: error.to_string(),
        };
        let answer = self.outbound.exchange(request).await.map_err(unreachable)?;
        let status = answer.status().as_u16();
        let body = answer.into_body();
        match status {
            200..=299 => Ok(body),
            // Unsplash answers 403 with these words when the hour's
            // requests are used up.
            403 if body.starts_with(b"Rate Limit Exceeded") => Err(StockError::Busy),
            401 | 403 => Err(StockError::KeyRefused),
            404 => Err(StockError::NotFound),
            429 => Err(StockError::Busy),
            status => Err(StockError::Unreachable {
                reason: format!("it answered {status}"),
            }),
        }
    }
}

/// `pairs` written as a query string.
fn encode(pairs: &[(&str, String)]) -> String {
    let mut query = form_urlencoded::Serializer::new(String::new());
    for (name, value) in pairs {
        query.append_pair(name, value);
    }
    query.finish()
}
