//! The Admin API under `/api/admin/v1/`.

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::{Slug, UserMessage};
use khalid_lint::Report;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::html_source::markdown_from_html;
use crate::keys::{KeyStore, Principal, Scope};
use crate::post_store::{CreatePostError, UpdatePostError};
use crate::preview_link::PreviewLink;
use crate::problem::{Problem, ProblemType};
use crate::publish_door::{PublishDoor, ScheduleError};
use crate::resources::{AdminPost, NewPost, PostDraft, PostEdit};

/// Where the Admin API lives.
pub(crate) const BASE: &str = "/api/admin/v1";

/// The door every post route reads and writes through.
type Posts = Arc<PublishDoor>;

/// The Admin API routes, each declared with `utoipa::path` and registered
/// through `routes!` (RULES.md BL-API-4). Every route needs an
/// integration key or staff token, and checks its own scope.
pub(crate) fn routes(posts: Posts, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(create_admin_post))
        .routes(routes!(read_admin_post, edit_admin_post))
        .routes(routes!(make_preview_link))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(posts)
}

/// Posts as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminPosts {
    /// The post or posts.
    posts: Vec<AdminPost>,
}

/// Reads one post in any status. Needs `posts:write`.
#[utoipa::path(
    get, path = "/posts/{id}/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    responses(
        (status = 200, description = "The post", body = AdminPosts),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_admin_post(
    State(posts): State<Posts>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<AdminPosts>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let post = posts
        .get(&id)
        .await
        .map_err(|error| Problem::internal(&error))?;
    let post = post.ok_or_else(|| missing(uri.path()))?;
    Ok(Json(AdminPosts { posts: vec![post] }))
}

/// An edit, wrapped the way the Admin API wraps posts: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditPosts {
    /// The one edit.
    posts: Vec<PostEdit>,
}

/// Edits one post, and publishes it with `"status": "published"` once
/// the linter lets it through, or with `"status": "scheduled"` and a
/// future `published_at` at that time. The edit carries the `updated_at` of the copy it was
/// made on; when the post changed since, the answer is a 409 naming who
/// changed it and when (RULES.md BL-API-7). Needs `posts:write`.
#[utoipa::path(
    put, path = "/posts/{id}/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    request_body = EditPosts,
    responses(
        (status = 200, description = "The post as saved", body = AdminPosts),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The post changed after the copy the edit was made on", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read, the schedule cannot be kept, or the linter refused to publish the post", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn edit_admin_post(
    State(posts): State<Posts>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<EditPosts>, JsonRejection>,
) -> Result<Json<AdminPosts>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Json(body) = body.map_err(|rejection| invalid_body(&rejection.body_text()))?;
    let [edit] = <[PostEdit; 1]>::try_from(body.posts)
        .map_err(|_| invalid_body("posts must hold exactly one post"))?;
    match posts.edit(&id, edit, principal.id().as_str()).await {
        Ok(post) => Ok(Json(AdminPosts { posts: vec![post] })),
        Err(UpdatePostError::NotFound) => Err(missing(uri.path())),
        Err(UpdatePostError::Stale {
            updated_at,
            updated_by,
        }) => Err(stale(updated_at, updated_by.as_deref())),
        Err(UpdatePostError::Refused(report)) => Err(refused(report)),
        Err(UpdatePostError::Schedule(error)) => Err(unschedulable(error)),
        Err(UpdatePostError::Store(error)) => Err(Problem::internal(&error)),
    }
}

/// The 422 for a post the linter refused to publish, with every finding.
fn refused(report: Report) -> Problem {
    let message = UserMessage::new(
        "The linter found something that stops this post from going live.",
        "Each finding under lint says what it is and why it matters, and the post stays a draft.",
        "Fix each finding marked block, then publish again.",
    );
    Problem::new(ProblemType::LintBlocked, &message).with_lint(report)
}

/// Preview links, wrapped the way the Admin API wraps resources.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PreviewLinks {
    /// The one new link.
    preview_links: Vec<PreviewLink>,
}

/// Makes a link that shows the post, draft or not, in the site's design
/// to anyone who has it, until it expires: 7 days unless `site.toml` says
/// otherwise. The page behind it tells search engines not to list it.
/// Needs `posts:write`.
#[utoipa::path(
    post, path = "/posts/{id}/preview-link/", tag = "admin posts",
    security(("admin_key" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    responses(
        (status = 201, description = "The new link and when it expires", body = PreviewLinks),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No post has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "This site makes no preview links", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn make_preview_link(
    State(posts): State<Posts>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<(StatusCode, Json<PreviewLinks>), Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Some(signer) = posts.previews() else {
        let message = UserMessage::new(
            "This site makes no preview links.",
            "A preview link needs the site's server, which signs each link.",
            "Ask for the link from the running khalid serve.",
        );
        return Err(Problem::new(ProblemType::InvalidBody, &message));
    };
    let found = posts.get(&id).await;
    found
        .map_err(|error| Problem::internal(&error))?
        .ok_or_else(|| missing(uri.path()))?;
    let link = signer.link(&id).ok_or_else(|| missing(uri.path()))?;
    let links = PreviewLinks {
        preview_links: vec![link],
    };
    Ok((StatusCode::CREATED, Json(links)))
}

/// The 422 for an edit whose schedule cannot be kept.
fn unschedulable(error: ScheduleError) -> Problem {
    let message = UserMessage::new(
        format!("The schedule in this edit cannot be kept: {error}."),
        "A post goes live or comes down only at a time still to come, and nothing was changed.",
        "Send status scheduled with a later published_at, and any unpublish_at after it.",
    );
    Problem::new(ProblemType::InvalidBody, &message).with_rule("published_at")
}

/// A new post, wrapped the way the Admin API wraps posts: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewPosts {
    /// The one new post.
    posts: Vec<NewPost>,
}

/// The longest `Idempotency-Key` accepted.
const MAX_IDEMPOTENCY_KEY: usize = 255;

/// What a new post's body is written in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Source {
    /// Markdown, stored as sent.
    #[default]
    Markdown,
    /// HTML, sanitised and turned into markdown.
    Html,
}

/// The query of a create request.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct CreateOptions {
    /// `html` to send the body as HTML; markdown otherwise.
    #[param(inline)]
    source: Option<Source>,
}

/// Creates a draft post from markdown, or from HTML with `source=html`. A
/// retry with the same `Idempotency-Key` returns the post the first
/// request made, so it never makes two (RULES.md BL-API-7). Needs
/// `posts:write`.
#[utoipa::path(
    post, path = "/posts/", tag = "admin posts",
    security(("admin_key" = [])),
    params(CreateOptions, ("Idempotency-Key" = Option<String>, Header, description = "Any text up to 255 characters that names this request, so a retry makes no second post")),
    request_body = NewPosts,
    responses(
        (status = 201, description = "The new draft", body = AdminPosts),
        (status = 400, description = "The source is neither markdown nor html", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "Another post has the slug", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_admin_post(
    State(posts): State<Posts>,
    Extension(principal): Extension<Principal>,
    options: Result<Query<CreateOptions>, QueryRejection>,
    headers: HeaderMap,
    body: Result<Json<NewPosts>, JsonRejection>,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<AdminPosts>), Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Query(options) = options.map_err(|_| unknown_source())?;
    let Json(body) = body.map_err(|rejection| invalid_body(&rejection.body_text()))?;
    let [new] = <[NewPost; 1]>::try_from(body.posts)
        .map_err(|_| invalid_body("posts must hold exactly one post"))?;
    let idempotency = idempotency_key(&headers)?;
    let draft = checked(new, options.source.unwrap_or_default()).await?;
    let made = posts
        .posts()
        .create(draft, principal.id().as_str(), idempotency)
        .await;
    let post = made.map_err(|error| match error {
        CreatePostError::SlugTaken(slug) => slug_taken(slug.as_str()),
        CreatePostError::NoSlug => {
            invalid_body("the title has no letters or digits to make a slug from")
        }
        CreatePostError::Store(error) => Problem::internal(&error),
    })?;
    let location = format!("{BASE}/posts/{}/", post.id());
    Ok((
        StatusCode::CREATED,
        [(LOCATION, location)],
        Json(AdminPosts { posts: vec![post] }),
    ))
}

/// Checks a new post: a title, a slug that is valid if given, and a body
/// in the format `source` names. HTML is sanitised and turned into
/// markdown off the async runtime, since a large body takes real work
/// (RULES.md BL-RUN-7).
async fn checked(new: NewPost, source: Source) -> Result<PostDraft, Problem> {
    if new.title.trim().is_empty() {
        return Err(invalid_body("title must not be empty"));
    }
    let slug = new
        .slug
        .as_deref()
        .map(Slug::try_new)
        .transpose()
        .map_err(|_| invalid_body("slug must be lowercase letters, digits and hyphens"))?;
    let author = new
        .author
        .as_deref()
        .map(Slug::try_new_unicode)
        .transpose()
        .map_err(|_| invalid_body("author must be an author's slug, such as asif"))?;
    let markdown = match (source, new.markdown, new.html) {
        (Source::Markdown, Some(markdown), None) => markdown,
        (Source::Html, None, Some(html)) => {
            tokio::task::spawn_blocking(move || markdown_from_html(&html))
                .await
                .map_err(|error| Problem::internal(&error))?
                .map_err(|_| invalid_body("the HTML could not be read"))?
        }
        (Source::Html, _, _) => return Err(invalid_body("source=html needs html and no markdown")),
        (Source::Markdown, _, _) => {
            return Err(invalid_body(
                "markdown is required, and html needs source=html",
            ));
        }
    };
    Ok(PostDraft::new(&new.title, slug, &markdown).with_author(author))
}

/// The 400 for a `source` that is neither `markdown` nor `html`.
fn unknown_source() -> Problem {
    let message = UserMessage::new(
        "The source parameter is neither markdown nor html.",
        "The server cannot tell how to read the body.",
        "Use source=html for HTML, or leave source out for markdown.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("source")
}

/// The `Idempotency-Key` header, if sent: 1 to 255 visible characters.
fn idempotency_key(headers: &HeaderMap) -> Result<Option<&str>, Problem> {
    let Some(value) = headers.get("idempotency-key") else {
        return Ok(None);
    };
    match value.to_str() {
        Ok(key) if !key.is_empty() && key.len() <= MAX_IDEMPOTENCY_KEY => Ok(Some(key)),
        _ => Err(invalid_body(
            "Idempotency-Key must be 1 to 255 visible characters",
        )),
    }
}

fn slug_taken(slug: &str) -> Problem {
    let message = UserMessage::new(
        format!("Another post already has the slug {slug}."),
        "Two posts cannot share one address.",
        "Send a different slug, or leave slug out to have one made from the title.",
    );
    Problem::new(ProblemType::Conflict, &message).with_rule("slug")
}

fn invalid_body(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("The request body could not be read: {reason}."),
        "Nothing was changed.",
        "Send JSON shaped like the example in /api/openapi.json, with updated_at.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

fn stale(updated_at: DateTime<Utc>, updated_by: Option<&str>) -> Problem {
    let when = updated_at.format("%Y-%m-%d %H:%M UTC");
    let message = UserMessage::new(
        format!(
            "{} changed this post at {when}, after the copy you edited.",
            updated_by.unwrap_or("Someone")
        ),
        "Saving your copy would undo their change.",
        "Fetch the post again, make your change to it, and send its new updated_at.",
    );
    Problem::new(ProblemType::Conflict, &message).with_rule("updated_at")
}

fn missing(path: &str) -> Problem {
    let message = UserMessage::new(
        "No post has this identity.",
        "It may have been deleted, or the identity may be mistyped.",
        "Check the identity, or list the posts to find it.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
