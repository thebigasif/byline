//! Free photos and GIFs for the editor, `/api/admin/v1/media/search/`
//! (plan 4.6, KH-EDT-1, decisions E6 to E8 in
//! `docs/decisions/2026-10-31-editor-emoji-photos-and-edits.md`).

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::{Extension, Json};
use khalid_core::UserMessage;
use khalid_media::{FoundPicture, PictureSource, StockError, StockSearch};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::admin_api::invalid_body;
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

/// A photo from Unsplash that went into a post.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UsedPhoto {
    /// The photo's identity at Unsplash, as the search gave it.
    id: String,
}

/// Tells Unsplash a photo went into a post, as its API terms ask whenever
/// a photo is used. The editor sends it when the writer picks a photo.
/// Needs `media:write`.
#[utoipa::path(
    post, path = "/media/search/used/", tag = "admin media",
    security(("admin_key" = [])),
    request_body = UsedPhoto,
    responses(
        (status = 200, description = "Unsplash was told", body = UsedPhoto),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "Unsplash has no photo with this identity", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
        (status = 429, description = "Unsplash asks to wait", body = Problem, content_type = "application/problem+json"),
        (status = 503, description = "Photo search is off, has no key, or Unsplash did not answer", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn photo_used(
    State(media): State<Option<MediaLibrary>>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<UsedPhoto>, JsonRejection>,
) -> Result<Json<UsedPhoto>, Problem> {
    require(&principal, Scope::MediaWrite)?;
    let Json(body) = body.map_err(|rejection| invalid_body(&rejection.body_text()))?;
    let unsplash = StockSource::Unsplash;
    let search = media.as_ref().and_then(MediaLibrary::search);
    let search = search.ok_or_else(|| failed(unsplash, &StockError::Off))?;
    search
        .used(&body.id)
        .await
        .map_err(|error| failed(unsplash, &error))?;
    Ok(Json(body))
}

/// The problem for `error`, in plain words naming `source`.
fn failed(source: StockSource, error: &StockError) -> Problem {
    let name = source.name();
    let (what, why, fix, kind) = match error {
        StockError::Off => (
            format!("{name} is turned off on this blog."),
            "The owner decides whether writers can find pictures there.".to_owned(),
            format!(
                "Ask the owner to turn it on under [media] in site.toml. {}",
                source_setting(source)
            ),
            ProblemType::Unavailable,
        ),
        StockError::NoKey { variable } => (
            format!("{name} is on, but its key is not set."),
            "The blog needs the key to ask for pictures.".to_owned(),
            format!("Set {variable} in the server's environment and start khalid serve again."),
            ProblemType::Unavailable,
        ),
        StockError::KeyRefused => (
            format!("{name} refused the blog's key."),
            "The key may be mistyped, or the app it belongs to was turned off.".to_owned(),
            format!("Check the key in {}.", source.picture_source().variable()),
            ProblemType::Unavailable,
        ),
        StockError::Busy => (
            format!("{name} asks the blog to wait."),
            "Each key may only ask a set number of times an hour.".to_owned(),
            "Try again in a few minutes.".to_owned(),
            ProblemType::TooManyRequests,
        ),
        StockError::NotFound => (
            "The source has no picture with this identity.".to_owned(),
            "It may have been removed.".to_owned(),
            "Search again and pick another picture.".to_owned(),
            ProblemType::NotFound,
        ),
        _ => (
            format!("{name} did not answer as expected."),
            "Its service may be down for a moment.".to_owned(),
            "Try again in a minute.".to_owned(),
            ProblemType::Unavailable,
        ),
    };
    Problem::new(kind, &UserMessage::new(what, why, fix)).with_rule("media.search")
}

/// The line in `site.toml` that turns `source` on.
fn source_setting(source: StockSource) -> &'static str {
    match source {
        StockSource::Unsplash => "The line is unsplash = true.",
        StockSource::Gifs => "The line is gifs = true.",
    }
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
