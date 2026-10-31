//! The `SerpDad` route of the Admin API under `/api/admin/v1/serpdad/`,
//! for the Connect to `SerpDad` page (KH-API-6,
//! `docs/decisions/2026-10-31-connect-to-serpdad.md`). Every method needs
//! `settings:write`.

use std::error::Error;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyHashing, KeyKind, KeyStore, KeyToken, Principal, Scope, StoredKey};
use crate::problem::{Problem, ProblemType};
use crate::resources::LintCounts;
use crate::serpdad_store::{ArrivedDraft, ReceivedDraft, SerpdadLink, SerpdadStore};

/// The name the key `SerpDad` sends drafts with is stored under.
const KEY_NAME: &str = "SerpDad";

/// The site's `SerpDad` connection, or `None` for a site with no database,
/// and the keys its key is made in.
#[derive(Clone)]
pub(crate) struct Serpdad {
    store: Option<Arc<dyn SerpdadStore>>,
    keys: Arc<dyn KeyStore>,
}

impl Serpdad {
    /// The connection kept in `store`, making keys in `keys`.
    pub(crate) fn new(store: Option<Arc<dyn SerpdadStore>>, keys: Arc<dyn KeyStore>) -> Self {
        Self { store, keys }
    }

    /// Records `draft` when `principal` is `SerpDad`'s key. A record that
    /// fails is logged, never answered: the draft is already made.
    pub(crate) async fn arrived(&self, principal: &Principal, draft: ArrivedDraft) {
        let Some(store) = &self.store else {
            return;
        };
        let recorded = match store.link().await {
            Ok(Some(link)) if &link.key == principal.id() => store.record(&draft).await,
            Ok(_) => return,
            Err(failed) => Err(failed),
        };
        if let Err(failed) = recorded {
            let error: &(dyn Error + 'static) = &failed;
            tracing::error!(error, "a draft from SerpDad could not be recorded");
        }
    }
}

/// The `SerpDad` route, declared with `utoipa::path` (RULES.md BL-API-4),
/// behind an integration key or staff token.
pub(crate) fn routes(serpdad: Serpdad, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_serpdad, connect_serpdad, disconnect_serpdad))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .layer(Extension(serpdad))
}

/// The connection, or `null` while it is off.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SerpdadConnection {
    /// When the key `SerpDad` sends drafts with was made.
    connected_at: DateTime<Utc>,
    /// The key itself, to paste into `SerpDad`. Only the answer that makes
    /// it carries it, once.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(
        example = "ki_1111222233334444_aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111"
    )]
    key: Option<String>,
}

impl From<&SerpdadLink> for SerpdadConnection {
    fn from(link: &SerpdadLink) -> Self {
        Self {
            connected_at: link.connected_at,
            key: None,
        }
    }
}

/// A draft `SerpDad` sent.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SerpdadDraft {
    /// The post's identity.
    #[schema(example = "en.choosing-anchor-text")]
    id: String,
    /// Its title as it arrived.
    title: String,
    /// What the linter found in it on arrival, or `null` when it could not
    /// run.
    lint: Option<LintCounts>,
    /// When it arrived.
    received_at: DateTime<Utc>,
    /// Where it opens in the admin's editor.
    #[schema(example = "/admin/posts/en.choosing-anchor-text")]
    admin_path: String,
}

impl From<ReceivedDraft> for SerpdadDraft {
    fn from(received: ReceivedDraft) -> Self {
        let draft = received.draft;
        Self {
            admin_path: format!("/admin/posts/{}", draft.post),
            id: draft.post,
            title: draft.title,
            lint: draft.lint,
            received_at: received.received_at,
        }
    }
}

/// The `SerpDad` connection and the drafts it sent.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SerpdadState {
    /// The connection, or `null` while it is off.
    connection: Option<SerpdadConnection>,
    /// The last 20 drafts `SerpDad` sent, newest first, kept after the
    /// connection is turned off.
    drafts: Vec<SerpdadDraft>,
}

/// Reads whether `SerpDad` is connected and the last 20 drafts it sent.
/// Needs `settings:write`.
#[utoipa::path(
    get, path = "/serpdad/", tag = "admin serpdad",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "The connection and the drafts", body = SerpdadState),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_serpdad(
    Extension(serpdad): Extension<Serpdad>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<SerpdadState>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let Some(store) = serpdad.store else {
        let drafts = Vec::new();
        return Ok(Json(SerpdadState {
            connection: None,
            drafts,
        }));
    };
    let link = store.link().await.map_err(|e| Problem::internal(&e))?;
    let recent = store.recent().await.map_err(|e| Problem::internal(&e))?;
    let connection = link.as_ref().map(SerpdadConnection::from);
    let drafts = recent.into_iter().map(SerpdadDraft::from).collect();
    Ok(Json(SerpdadState { connection, drafts }))
}

/// Turns the connection on with a new key that may only write posts, and
/// revokes the key it replaces. The answer shows the key once; paste it
/// into `SerpDad` with the blog's address. Needs `settings:write`.
#[utoipa::path(
    post, path = "/serpdad/", tag = "admin serpdad",
    security(("admin_key" = [])),
    responses(
        (status = 201, description = "The connection, with its new key this once", body = SerpdadState),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The site keeps no database", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn connect_serpdad(
    Extension(serpdad): Extension<Serpdad>,
    Extension(principal): Extension<Principal>,
) -> Result<(StatusCode, Json<SerpdadState>), Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let store = serpdad.store.clone().ok_or_else(no_database)?;
    let (token, stored) = new_key().await?;
    let id = token.id().clone();
    serpdad
        .keys
        .create(stored, KEY_NAME)
        .await
        .map_err(|e| Problem::internal(&e))?;
    let replaced = match store.connect(&id).await {
        Ok(replaced) => replaced,
        Err(failed) => {
            revoke(serpdad.keys.as_ref(), &id).await;
            return Err(Problem::internal(&failed));
        }
    };
    if let Some(old) = replaced {
        revoke(serpdad.keys.as_ref(), &old).await;
    }
    let link = store.link().await.map_err(|e| Problem::internal(&e))?;
    let link = link.ok_or_else(|| Problem::internal(&NotKept))?;
    let recent = store.recent().await.map_err(|e| Problem::internal(&e))?;
    let connection = SerpdadConnection {
        key: Some(token.reveal().expose().clone()),
        ..SerpdadConnection::from(&link)
    };
    let drafts = recent.into_iter().map(SerpdadDraft::from).collect();
    let state = SerpdadState {
        connection: Some(connection),
        drafts,
    };
    Ok((StatusCode::CREATED, Json(state)))
}

/// Turns the connection off: `SerpDad`'s key is revoked at once, and the
/// drafts it sent stay. Needs `settings:write`.
#[utoipa::path(
    delete, path = "/serpdad/", tag = "admin serpdad",
    security(("admin_key" = [])),
    responses(
        (status = 204, description = "The connection is off"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "SerpDad is not connected", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn disconnect_serpdad(
    Extension(serpdad): Extension<Serpdad>,
    Extension(principal): Extension<Principal>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let old = match &serpdad.store {
        Some(store) => store
            .disconnect()
            .await
            .map_err(|e| Problem::internal(&e))?,
        None => None,
    };
    let Some(old) = old else {
        let message = UserMessage::new(
            "SerpDad is not connected.",
            "There is no SerpDad key to switch off.",
            "Nothing to do. Make a key on the SerpDad page to connect it.",
        );
        return Err(Problem::new(ProblemType::NotFound, &message));
    };
    revoke(serpdad.keys.as_ref(), &old).await;
    Ok(StatusCode::NO_CONTENT)
}

/// A new integration key with only `posts:write`, hashed off the async
/// runtime, since Argon2id takes real work (RULES.md BL-RUN-7).
async fn new_key() -> Result<(KeyToken, StoredKey), Problem> {
    let hashed = tokio::task::spawn_blocking(|| {
        let token = KeyToken::generate(KeyKind::Integration).map_err(|e| Problem::internal(&e))?;
        let scopes = vec![Scope::PostsWrite];
        let stored = StoredKey::for_integration(&token, scopes, &KeyHashing::new())
            .map_err(|e| Problem::internal(&e))?;
        Ok((token, stored))
    })
    .await;
    hashed.map_err(|e| Problem::internal(&e))?
}

/// Revokes `id`, logging a failure: the connection has already moved on.
async fn revoke(keys: &dyn KeyStore, id: &crate::keys::KeyId) {
    if let Err(failed) = keys.revoke(id).await {
        let error: &(dyn Error + 'static) = &failed;
        tracing::error!(
            error,
            key = id.as_str(),
            "an old SerpDad key could not be revoked"
        );
    }
}

/// The 422 for a site that keeps no database.
fn no_database() -> Problem {
    let message = UserMessage::new(
        "This site cannot connect to SerpDad here.",
        "SerpDad's key and the drafts it sends are kept by the site's server, in its database.",
        "Open the admin on the running khalid serve.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

/// The link just written could not be read back.
#[derive(Debug, thiserror::Error)]
#[error("the SerpDad key just stored was not found")]
struct NotKept;
