//! The Admin API's key routes: the site's API keys, listed, made and
//! revoked (plan sections 4.6 and 5.12, KH-API-2, ADR 0011).

use std::sync::Arc;

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_entries::Noun;
use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Access, KeyHashing, KeyKind, KeyStore, KeyToken, Principal, Scope, StoredKey};
use crate::problem::Problem;
use crate::resources::{AdminKey, NewKey, NewKeyKind};

/// How messages name a key.
const KEY: Noun = Noun {
    one: "key",
    many: "keys",
    example: "1111222233334444",
};

/// The site's keys, the same store every request's key is checked in.
type Keys = Arc<dyn KeyStore>;

/// The key routes, under the Admin API's base. A key opens the site, so
/// every route needs `admin:all`, which owners and admins hold.
pub(crate) fn routes(keys: Keys, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_keys, create_key))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(keys)
}

/// Keys as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminKeys {
    /// The key or keys.
    keys: Vec<AdminKey>,
}

/// Lists every key that works, oldest first, with what each may do and
/// never its secret. Needs `admin:all`.
#[utoipa::path(
    get, path = "/keys/", tag = "admin keys",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every key that works", body = AdminKeys),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks admin:all", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_keys(
    State(keys): State<Keys>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminKeys>, Problem> {
    require(&principal, Scope::AdminAll)?;
    let listed = keys
        .list()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let keys = listed.into_iter().map(AdminKey::from).collect();
    Ok(Json(AdminKeys { keys }))
}

/// A new key, wrapped the way the Admin API wraps keys: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewKeys {
    /// The one new key.
    keys: Vec<NewKey>,
}

/// Makes a content key or an integration key with the scopes it names.
/// The answer shows the key this once; keep it, because only its hash is
/// stored (RULES.md BL-SEC-3). Needs `admin:all`.
#[utoipa::path(
    post, path = "/keys/", tag = "admin keys",
    security(("admin_key" = [])),
    request_body = NewKeys,
    responses(
        (status = 201, description = "The new key, with the key itself this once", body = AdminKeys),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks admin:all", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, the name is empty, or the scopes do not fit the kind", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_key(
    State(keys): State<Keys>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewKeys>, JsonRejection>,
) -> Result<(StatusCode, Json<AdminKeys>), Problem> {
    require(&principal, Scope::AdminAll)?;
    let new = KEY.one(KEY.body(body)?.keys)?;
    KEY.filled("name", &new.name)?;
    let access = access(&new)?;
    let (token, stored) = tokio::task::spawn_blocking(move || made(access))
        .await
        .map_err(|error| Problem::internal(&error))??;
    let created = keys.create(stored, new.name.trim()).await;
    created.map_err(|error| Problem::internal(&error))?;
    let listed = keys
        .list()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let key = listed.into_iter().find(|key| &key.id == token.id());
    let key = key.ok_or_else(|| KEY.missing(""))?;
    let key = AdminKey::from(key).showing(&token.reveal());
    Ok((StatusCode::CREATED, Json(AdminKeys { keys: vec![key] })))
}

/// What a new key may do, when its scopes fit its kind: none for a
/// content key, at least one known scope for an integration key.
fn access(new: &NewKey) -> Result<Access, Problem> {
    let scopes: Result<Vec<Scope>, _> = new.scopes.iter().map(|s| s.parse::<Scope>()).collect();
    let scopes = scopes.map_err(|_| {
        KEY.invalid(
            "each scope must be one the API lists, such as posts:write",
            Some("scopes"),
        )
    })?;
    match (new.kind, scopes.is_empty()) {
        (NewKeyKind::Content, true) => Ok(Access::Content),
        (NewKeyKind::Integration, false) => Ok(Access::Integration(scopes)),
        (NewKeyKind::Content, false) => {
            Err(KEY.invalid("a content key takes no scopes", Some("scopes")))
        }
        (NewKeyKind::Integration, true) => Err(KEY.invalid(
            "an integration key needs at least one scope",
            Some("scopes"),
        )),
    }
}

/// A new random key with `access`, and what to store for it. Hashing is
/// slow on purpose, so this runs on a blocking thread (RULES.md BL-RUN-7).
fn made(access: Access) -> Result<(KeyToken, StoredKey), Problem> {
    let (kind, scopes) = match access {
        Access::Integration(scopes) => (KeyKind::Integration, Some(scopes)),
        _ => (KeyKind::Content, None),
    };
    let token = KeyToken::generate(kind).map_err(|error| Problem::internal(&error))?;
    let stored = match scopes {
        Some(scopes) => StoredKey::for_integration(&token, scopes, &KeyHashing::new())
            .map_err(|error| Problem::internal(&error))?,
        None => StoredKey::for_token(&token),
    };
    Ok((token, stored))
}
