//! The media library under `/api/admin/v1/media/` (plan section 5.15).

use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderName, StatusCode};
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use khalid_media::{
    ContentKey, EditMediaError, FocalPoint, Folder, Kind, Library, MediaEdit, MediaItem, Refusal,
    StockSearch, Upload, UploadMediaError, Uploaded,
};
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_api::BASE;
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::content_api::check_fields;
use crate::content_store::select;
use crate::keys::{Principal, Scope};
use crate::media_search;
use crate::pagination::Meta;
use crate::problem::{Problem, ProblemType};
use crate::query::{ListQuery, RawListQuery};
use crate::resources::{AdminMedia, MediaEditBody, MediaEdits, MediaItems, MediaList};

/// A site's media library, and the address its own files are served under.
#[derive(Debug, Clone)]
pub struct MediaLibrary {
    library: Arc<Library>,
    site: String,
    /// Photo and GIF search for the editor, when the server has it.
    pub(crate) search: Option<Arc<StockSearch>>,
}

impl MediaLibrary {
    /// `library`, whose files on the site itself are served under `site`,
    /// the blog's address with its mount, such as
    /// `https://www.example.com/blog/`.
    pub fn new(library: Arc<Library>, site: &str) -> Self {
        let site = format!("{}/", site.trim_end_matches('/'));
        Self {
            library,
            site,
            search: None,
        }
    }

    /// `item` as the Admin API sends it, with the full address readers
    /// fetch its file from.
    pub fn shown(&self, item: &MediaItem) -> AdminMedia {
        AdminMedia::new(item, self.library.address(item), &self.site)
    }
}

/// The library, or `None` on a router with no site behind it.
type Media = Option<MediaLibrary>;

/// The media routes. Each needs `media:write`.
pub(crate) fn routes(media: Media, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(upload_media, list_media))
        .routes(routes!(read_media, edit_media))
        .routes(routes!(media_search::search_pictures))
        .routes(routes!(media_search::photo_used))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(media)
}

/// Where to file an upload and what it was called.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct UploadOptions {
    /// The file's name, made a slug; the extension comes from its content.
    name: Option<String>,
    /// The library folder, such as `guides/2026`. The top when left out.
    folder: Option<String>,
}

/// Adds the file sent as the body to the media library; its type is read
/// from its bytes. A file the library already has answers 200 with that
/// item, so a retry adds no second copy. Needs `media:write`.
#[utoipa::path(
    post, path = "/media/", tag = "admin media",
    security(("admin_key" = [])),
    params(UploadOptions),
    request_body(content = Vec<u8>, content_type = "application/octet-stream", description = "The file's bytes"),
    responses(
        (status = 201, description = "The new item", body = MediaItems),
        (status = 200, description = "The library already had this file; the item as it is", body = MediaItems),
        (status = 400, description = "The folder is not up to five slugs joined by /", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The file is a type the library refuses, cannot be read, or is too large", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn upload_media(
    State(media): State<Media>,
    Extension(principal): Extension<Principal>,
    options: Result<Query<UploadOptions>, QueryRejection>,
    body: Body,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<MediaItems>), Problem> {
    require(&principal, Scope::MediaWrite)?;
    let media = media.ok_or_else(no_library)?;
    let Query(options) = options.map_err(|_| bad_folder())?;
    let folder = options.folder.as_deref().unwrap_or_default();
    let folder = Folder::try_new(folder).map_err(|_| bad_folder())?;
    let most = usize::try_from(Kind::Video.max_bytes()).unwrap_or(usize::MAX);
    let bytes = axum::body::to_bytes(body, most)
        .await
        .map_err(|_| refused(Refusal::TooLarge { kind: Kind::Video }))?;
    let name = options.name.as_deref().unwrap_or("file");
    let upload = Upload::new(name, folder, bytes.to_vec(), principal.id().as_str());
    let (status, item) = match media.library.upload(upload).await {
        Ok(Uploaded::New(item)) => (StatusCode::CREATED, item),
        Ok(Uploaded::Duplicate(item)) => (StatusCode::OK, item),
        Err(UploadMediaError::Refused(why)) => return Err(refused(why)),
        Err(error) => return Err(Problem::internal(&error)),
    };
    let location = format!("{BASE}/media/{}/", item.id);
    let items = MediaItems {
        media: vec![media.shown(&item)],
    };
    Ok((status, [(LOCATION, location)], Json(items)))
}

/// Lists the media library, newest upload first, with the shared list
/// grammar: `filter`, `order`, `limit` and `page` (RULES.md BL-API-5).
/// `filter=alt:null+kind:image` finds images still waiting for alt text.
/// Needs `media:write`.
#[utoipa::path(
    get, path = "/media/", tag = "admin media",
    security(("admin_key" = [])),
    params(RawListQuery),
    responses(
        (status = 200, description = "One page of the library", body = MediaList),
        (status = 400, description = "A list parameter could not be read", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_media(
    State(media): State<Media>,
    Extension(principal): Extension<Principal>,
    Query(raw): Query<RawListQuery>,
) -> Result<Json<MediaList>, Problem> {
    require(&principal, Scope::MediaWrite)?;
    let media = media.ok_or_else(no_library)?;
    let query = ListQuery::try_from(&raw)?;
    check_fields(&query, AdminMedia::FIELDS)?;
    let all: Vec<AdminMedia> = media
        .library
        .items()
        .iter()
        .map(|item| media.shown(item))
        .collect();
    let listed = select(all.iter(), &query);
    let meta = Meta::new(query.page(), query.limit(), listed.total());
    let media = listed.into_items();
    Ok(Json(MediaList { media, meta }))
}

/// Reads one item of the media library. Needs `media:write`.
#[utoipa::path(
    get, path = "/media/{id}/", tag = "admin media",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The item's identity")),
    responses(
        (status = 200, description = "The item", body = MediaItems),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No item has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_media(
    State(media): State<Media>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<MediaItems>, Problem> {
    require(&principal, Scope::MediaWrite)?;
    let media = media.ok_or_else(no_library)?;
    let item = ContentKey::parse(&id)
        .and_then(|id| media.library.get(&id))
        .ok_or_else(|| missing(uri.path()))?;
    let media = vec![media.shown(&item)];
    Ok(Json(MediaItems { media }))
}

/// Changes an item's alt text, caption, focal point or folder; a field left
/// out stays. The edit carries the `updated_at` of the copy it was made on,
/// and is a 409 when the item changed since (RULES.md BL-API-7). Needs
/// `media:write`.
#[utoipa::path(
    put, path = "/media/{id}/", tag = "admin media",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The item's identity")),
    request_body = MediaEdits,
    responses(
        (status = 200, description = "The item as saved", body = MediaItems),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No item has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The item changed after the copy the edit was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn edit_media(
    State(media): State<Media>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<MediaEdits>, JsonRejection>,
) -> Result<Json<MediaItems>, Problem> {
    require(&principal, Scope::MediaWrite)?;
    let media = media.ok_or_else(no_library)?;
    let Json(body) = body.map_err(|rejection| invalid(&rejection.body_text()))?;
    let [edit] = <[MediaEditBody; 1]>::try_from(body.media)
        .map_err(|_| invalid("media must hold exactly one edit"))?;
    let edit = checked(edit)?;
    let id = ContentKey::parse(&id).ok_or_else(|| missing(uri.path()))?;
    let by = principal.id().as_str();
    let item = match media.library.edit(&id, edit, by).await {
        Ok(item) => item,
        Err(EditMediaError::NotFound) => return Err(missing(uri.path())),
        Err(EditMediaError::Stale {
            updated_at,
            updated_by,
        }) => return Err(stale(updated_at, &updated_by)),
        Err(EditMediaError::TooLong { field, limit }) => {
            return Err(invalid(&format!(
                "{field} is longer than {limit} characters"
            )));
        }
        Err(error) => return Err(Problem::internal(&error)),
    };
    let media = vec![media.shown(&item)];
    Ok(Json(MediaItems { media }))
}

/// The edit `body` asks for, with its folder and focal point checked.
fn checked(body: MediaEditBody) -> Result<MediaEdit, Problem> {
    let mut edit = MediaEdit::new(body.updated_at);
    edit.alt = body.alt;
    edit.caption = body.caption;
    edit.folder = body
        .folder
        .as_deref()
        .map(Folder::try_new)
        .transpose()
        .map_err(|_| invalid("folder must be up to five slugs joined by /"))?;
    edit.focal_point = body
        .focal_point
        .map(|point| FocalPoint::try_new(point.x, point.y))
        .transpose()
        .map_err(|_| invalid("focal_point x and y must each be 0 to 1000"))?;
    Ok(edit)
}

fn invalid(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("The edit could not be read: {reason}."),
        "Nothing was changed.",
        "Send JSON shaped like the example in /api/openapi.json, with updated_at.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

fn stale(updated_at: DateTime<Utc>, updated_by: &str) -> Problem {
    let when = updated_at.format("%Y-%m-%d %H:%M UTC");
    let message = UserMessage::new(
        format!("{updated_by} changed this item at {when}, after the copy you edited."),
        "Saving your copy would undo their change.",
        "Fetch the item again, make your change to it, and send its new updated_at.",
    );
    Problem::new(ProblemType::Conflict, &message).with_rule("updated_at")
}

fn missing(path: &str) -> Problem {
    let message = UserMessage::new(
        "No media item has this identity.",
        "It may never have been uploaded, or the identity may be mistyped.",
        "Check the identity, or list the media library to find it.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}

/// The 422 for a file the library refuses, saying why in plain words.
fn refused(why: Refusal) -> Problem {
    let (because, fix) = match why {
        Refusal::TooLarge { .. } => (
            "Each kind of file has a size limit: 20 MB for images and files, 50 MB for audio and 100 MB for video.",
            "Make the file smaller, or put a long video on a video host and embed it in the post.",
        ),
        Refusal::Broken => (
            "The file starts like an image, but its pixels cannot be read, so it may be damaged.",
            "Open the image in an image editor, save it again as JPEG, PNG or WebP, and upload that.",
        ),
        _ => (
            "The library keeps only types it can check by their content, so no file can carry a program or a script.",
            "Send a JPEG, PNG or WebP image, an MP4 or WebM video, MP3, M4A, Ogg, WAV or FLAC audio, or a PDF, ZIP or text file.",
        ),
    };
    let message = UserMessage::new(format!("The file was not added: {why}."), because, fix);
    Problem::new(ProblemType::InvalidBody, &message).with_rule("media")
}

fn bad_folder() -> Problem {
    let message = UserMessage::new(
        "The folder could not be read.",
        "A folder is up to five names of lowercase letters, digits and hyphens, joined by /.",
        "Send a folder such as guides/2026, or leave folder out for the top of the library.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("folder")
}

fn no_library() -> Problem {
    let message = UserMessage::new(
        "This site has no media library.",
        "The library needs the site's server, which keeps the files.",
        "Send the request to the running khalid serve.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}
