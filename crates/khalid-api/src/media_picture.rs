//! A turned or cropped copy of a library picture, `/api/admin/v1/media/{id}/copies/`
//! (plan 4.6, KH-EDT-1, decisions E9 and E10).

use axum::extract::rejection::JsonRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderName, StatusCode};
use axum::{Extension, Json};
use khalid_core::UserMessage;
use khalid_media::{ContentKey, CropBox, EditPictureError, PictureEdit, Turn, Uploaded};

use crate::admin_api::BASE;
use crate::admin_key::require;
use crate::keys::{Principal, Scope};
use crate::media_api::{MediaLibrary, missing, no_library};
use crate::problem::{Problem, ProblemType};
use crate::resources::{MediaItems, PictureEditBody};

/// Makes a copy of a picture, turned and then cropped, as a new item in
/// the same folder with the same name, alt text and caption. The original
/// stays, so posts that show it do not change. The same edit made again
/// answers 200 with the first copy. Needs `media:write`.
#[utoipa::path(
    post, path = "/media/{id}/copies/", tag = "admin media",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The picture's identity")),
    request_body = PictureEditBody,
    responses(
        (status = 201, description = "The new copy", body = MediaItems),
        (status = 200, description = "The copy this edit already made", body = MediaItems),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks media:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No item has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The edit could not be read, changes nothing, or the item is not a picture", body = Problem, content_type = "application/problem+json"),
    )
)]
pub(crate) async fn copy_picture(
    State(media): State<Option<MediaLibrary>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<PictureEditBody>, JsonRejection>,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<MediaItems>), Problem> {
    require(&principal, Scope::MediaWrite)?;
    let media = media.ok_or_else(no_library)?;
    let Json(body) = body.map_err(|rejection| unreadable(&rejection.body_text()))?;
    let edit = checked(body)?;
    let id = ContentKey::parse(&id).ok_or_else(|| missing(uri.path()))?;
    let by = principal.id().as_str();
    let (status, copy) = match media.library.edit_picture(&id, edit, by).await {
        Ok(Uploaded::New(copy)) => (StatusCode::CREATED, copy),
        Ok(Uploaded::Duplicate(copy)) => (StatusCode::OK, copy),
        Err(EditPictureError::NotFound) => return Err(missing(uri.path())),
        Err(EditPictureError::NotAPicture) => return Err(not_a_picture()),
        Err(error) => return Err(Problem::internal(&error)),
    };
    let location = format!("{BASE}/media/{}/", copy.id);
    let items = MediaItems {
        media: vec![media.shown(&copy)],
    };
    Ok((status, [(LOCATION, location)], Json(items)))
}

/// The edit `body` asks for, with its turn and box checked.
fn checked(body: PictureEditBody) -> Result<PictureEdit, Problem> {
    let turn = Turn::from_degrees(body.turn)
        .ok_or_else(|| unreadable("turn must be 0, 90, 180 or 270"))?;
    let crop = body
        .crop
        .map(|crop| CropBox::try_new(crop.left, crop.top, crop.width, crop.height))
        .transpose()
        .map_err(|_| unreadable("crop must be a box inside the picture, in thousandths"))?;
    if turn == Turn::None && crop.is_none() {
        return Err(unreadable("the edit must turn or crop the picture"));
    }
    Ok(PictureEdit::new(turn, crop))
}

fn unreadable(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("The edit could not be read: {reason}."),
        "No copy was made, and the picture stays as it is.",
        "Send turn in degrees, a crop box in thousandths, or both, as in /api/openapi.json.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

fn not_a_picture() -> Problem {
    let message = UserMessage::new(
        "Only pictures can be turned or cropped.",
        "This item is a file, a video or a sound.",
        "Pick a JPEG, PNG or WebP picture from the library.",
    );
    Problem::new(ProblemType::InvalidBody, &message).with_rule("media")
}
