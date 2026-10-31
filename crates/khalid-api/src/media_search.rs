//! Free photos and GIFs for the editor, `/api/admin/v1/media/search/`
//! (plan 4.6, KH-EDT-1, decisions E6 to E8 in
//! `docs/decisions/2026-10-31-editor-emoji-photos-and-edits.md`).

use axum::extract::State;
use axum::{Extension, Json};
use khalid_media::{PictureSource, StockSearch};
use serde::Serialize;
use utoipa::ToSchema;

use crate::admin_key::require;
use crate::keys::{Principal, Scope};
use crate::media_api::MediaLibrary;
use crate::problem::Problem;

/// Which searches are on.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PictureSearch {
    /// Whether the owner turned photo search on Unsplash on.
    unsplash: bool,
    /// Whether the owner turned GIF search on GIPHY on.
    gifs: bool,
}

/// Says which of photo search on Unsplash and GIF search on GIPHY the
/// owner turned on in `site.toml`, so the editor offers only those. Asks
/// neither. Needs `media:write`.
#[utoipa::path(
    get, path = "/media/search/", tag = "admin media",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Which searches are on", body = PictureSearch),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn search_pictures(
    State(media): State<Option<MediaLibrary>>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<PictureSearch>, Problem> {
    require(&principal, Scope::MediaWrite)?;
    let search = media.as_ref().and_then(MediaLibrary::search);
    let on = |source: PictureSource| search.is_some_and(|search| search.is_on(source));
    Ok(Json(PictureSearch {
        unsplash: on(PictureSource::Unsplash),
        gifs: on(PictureSource::Giphy),
    }))
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
