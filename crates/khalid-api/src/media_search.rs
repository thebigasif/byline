//! Free photos and GIFs for the editor, `/api/admin/v1/media/search/`
//! (plan 4.6, KH-EDT-1, decisions E6 to E8 in
//! `docs/decisions/2026-10-31-editor-emoji-photos-and-edits.md`).

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::{Extension, Json};
use khalid_core::UserMessage;
use khalid_media::{FoundPicture, PictureSource, StockError, StockSearch};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::admin_key::require;
use crate::keys::{Principal, Scope};
use crate::media_api::MediaLibrary;
use crate::problem::{Problem, ProblemType};

/// Where to look for a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StockSource {
    /// Free photos on Unsplash.
    Unsplash,
    /// GIFs on GIPHY.
    Gifs,
}

impl StockSource {
    fn picture_source(self) -> PictureSource {
        match self {
            Self::Unsplash => PictureSource::Unsplash,
            Self::Gifs => PictureSource::Giphy,
        }
    }

    /// The source's name as writers know it.
    fn name(self) -> &'static str {
        match self {
            Self::Unsplash => "Photo search on Unsplash",
            Self::Gifs => "GIF search on GIPHY",
        }
    }
}

/// What to look for.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SearchOptions {
    /// `unsplash` or `gifs`.
    source: Option<StockSource>,
    /// The words to look for. Left out, the answer only says which
    /// searches are on, without asking anyone.
    words: Option<String>,
    /// The page of results, from 1.
    page: Option<u32>,
}

/// Which searches are on, and the pictures found.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PictureSearch {
    /// Whether the owner turned photo search on Unsplash on.
    unsplash: bool,
    /// Whether the owner turned GIF search on GIPHY on.
    gifs: bool,
    /// The pictures found, 24 a page; empty without words.
    pictures: Vec<StockPicture>,
}

/// A photo or GIF found for a post. It stays where it came from: the post
/// shows it from `address`, with `credit` linked to `credit_link` under it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct StockPicture {
    /// Where it comes from.
    source: StockSource,
    /// Its identity at the source.
    id: String,
    /// Words its source has for it, a start for alt text; may be empty.
    description: String,
    /// A small copy for the list of results.
    preview: String,
    /// The address to put in the post.
    address: String,
    /// The width of the picture at `address`, in pixels.
    width: u32,
    /// The height of the picture at `address`, in pixels.
    height: u32,
    /// Who made it, or `GIPHY`.
    credit: String,
    /// The maker's page.
    credit_link: String,
    /// The picture's own page at its source.
    page: String,
}

impl StockPicture {
    fn new(source: StockSource, found: FoundPicture) -> Self {
        Self {
            source,
            id: found.id,
            description: found.description,
            preview: found.preview,
            address: found.address,
            width: found.width,
            height: found.height,
            credit: found.credit,
            credit_link: found.credit_link,
            page: found.page,
        }
    }
}

/// Finds free photos on Unsplash or GIFs on GIPHY for a post, through the
/// blog, so the keys never reach the browser. Each search is off until the
/// owner turns it on in `site.toml`; without `words` the answer only says
/// which are on. Needs `media:write`.
#[utoipa::path(
    get, path = "/media/search/", tag = "admin media",
    security(("admin_key" = [])),
    params(SearchOptions),
    responses(
        (status = 200, description = "Which searches are on, and the pictures found", body = PictureSearch),
        (status = 400, description = "The source or page could not be read", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
        (status = 429, description = "The source asks to wait before the next search", body = Problem, content_type = "application/problem+json"),
        (status = 503, description = "The search is off, has no key, or its source did not answer", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn search_pictures(
    State(media): State<Option<MediaLibrary>>,
    Extension(principal): Extension<Principal>,
    options: Result<Query<SearchOptions>, QueryRejection>,
) -> Result<Json<PictureSearch>, Problem> {
    require(&principal, Scope::MediaWrite)?;
    let Query(options) = options.map_err(|_| bad_search())?;
    let search = media.as_ref().and_then(MediaLibrary::search);
    let on = |source: PictureSource| search.is_some_and(|search| search.is_on(source));
    let mut answer = PictureSearch {
        unsplash: on(PictureSource::Unsplash),
        gifs: on(PictureSource::Giphy),
        pictures: Vec::new(),
    };
    let words = options.words.as_deref().map(str::trim).unwrap_or_default();
    if words.is_empty() {
        return Ok(Json(answer));
    }
    let source = options.source.ok_or_else(bad_search)?;
    let Some(search) = search else {
        return Err(failed(source, &StockError::Off));
    };
    let found = search
        .search(source.picture_source(), words, options.page.unwrap_or(1))
        .await
        .map_err(|error| failed(source, &error))?;
    answer.pictures = found
        .into_iter()
        .map(|found| StockPicture::new(source, found))
        .collect();
    Ok(Json(answer))
}

/// The problem for `error` while searching `source`.
fn failed(source: StockSource, error: &StockError) -> Problem {
    let kind = match error {
        StockError::Busy => ProblemType::TooManyRequests,
        _ => ProblemType::Unavailable,
    };
    let message = UserMessage::new(
        format!("{} did not work: {error}.", source.name()),
        "The blog asks for pictures for you, with the owner's key.",
        "Try again in a minute, or ask the owner to check [media] in site.toml and the key.",
    );
    Problem::new(kind, &message).with_rule("media.search")
}

fn bad_search() -> Problem {
    let message = UserMessage::new(
        "The search could not be read.",
        "A search needs a source, unsplash or gifs, and a page from 1.",
        "Send source=unsplash&words=mountain%20lake, for example.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("media.search")
}

impl MediaLibrary {
    /// The library with photo and GIF search through `search`.
    #[must_use]
    pub fn with_search(mut self, search: StockSearch) -> Self {
        self.search = Some(std::sync::Arc::new(search));
        self
    }

    /// Photo and GIF search, when the server has it.
    pub(crate) fn search(&self) -> Option<&StockSearch> {
        self.search.as_deref()
    }
}
