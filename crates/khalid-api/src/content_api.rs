//! The Content API under `/api/content/v1/`, and the `OpenAPI` document.

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, Query, State};
use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use khalid_core::{Slug, UserMessage};
use serde::Serialize;
use utoipa::openapi::OpenApi as Document;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::content_key::require_content_key;
use crate::content_store::{ContentStore, PostKey};
use crate::keys::KeyStore;
use crate::pagination::Meta;
use crate::problem::{Problem, ProblemType};
use crate::query::{ListQuery, RawListQuery};
use crate::resources::Post;

/// Where the Content API lives.
const BASE: &str = "/api/content/v1";

/// The store every route reads from.
type Store = Arc<dyn ContentStore>;

/// The document's title, the schemas every route shares and the ways a
/// content key may be sent.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Khalid API",
        description = "The Content API and the Admin API of a Khalid blog."
    ),
    components(schemas(Problem)),
    modifiers(&ContentKeySchemes)
)]
struct ApiDoc;

/// Adds the two ways to send a content key to the document.
struct ContentKeySchemes;

impl Modify for ContentKeySchemes {
    fn modify(&self, document: &mut Document) {
        let components = document.components.get_or_insert_with(Default::default);
        let header = ApiKeyValue::with_description(
            "Authorization",
            "The site's content key, as Khalid-Content followed by the key.",
        );
        components.add_security_scheme(
            "content_key",
            SecurityScheme::ApiKey(ApiKey::Header(header)),
        );
        let query = ApiKeyValue::with_description("key", "The site's content key.");
        components.add_security_scheme(
            "content_key_query",
            SecurityScheme::ApiKey(ApiKey::Query(query)),
        );
    }
}

/// The Content API routes and `/api/openapi.json`, with the `OpenAPI`
/// document the routes declare (RULES.md BL-API-4). Every route is
/// declared with `utoipa::path` and registered through `routes!`, so a
/// route cannot exist without its entry, and every route needs a content
/// key. The document itself needs none.
pub fn router(store: Store, keys: Arc<dyn KeyStore>) -> (Router, Document) {
    let content = OpenApiRouter::new()
        .routes(routes!(browse_posts))
        .routes(routes!(read_post))
        .routes(routes!(read_post_by_slug))
        .route_layer(middleware::from_fn_with_state(keys, require_content_key))
        .with_state(store);
    let (router, document) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(BASE, content)
        .split_for_parts();
    let served = Json(document.clone());
    let router = router.route("/api/openapi.json", get(move || async move { served }));
    (router, document)
}

/// Posts, as every post response wraps them (`docs/standards/api.md`).
#[derive(Debug, Serialize, ToSchema)]
pub struct Posts {
    /// The posts on this page, or the one post asked for.
    posts: Vec<Post>,
    /// Pagination, on lists only.
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Meta>,
}

/// Lists published posts.
#[utoipa::path(
    get, path = "/posts/", tag = "posts", params(RawListQuery), security(("content_key" = []), ("content_key_query" = [])),
    responses(
        (status = 200, description = "One page of published posts", body = Posts),
        (status = 400, description = "A list parameter could not be read", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn browse_posts(
    State(store): State<Store>,
    Query(raw): Query<RawListQuery>,
) -> Result<Json<Posts>, Problem> {
    let query = ListQuery::try_from(&raw)?;
    check_fields(&query)?;
    let listed = store
        .posts(&query)
        .await
        .map_err(|error| Problem::internal(&error))?;
    let meta = Meta::new(query.page(), query.limit(), listed.total());
    let formats = query.formats();
    let posts = listed
        .into_items()
        .into_iter()
        .map(|post| post.with_formats(formats));
    Ok(Json(Posts {
        posts: posts.collect(),
        meta: Some(meta),
    }))
}

/// Reads one published post by its identity.
#[utoipa::path(
    get, path = "/posts/{id}/", tag = "posts", security(("content_key" = []), ("content_key_query" = [])),
    params(("id" = String, Path, description = "The post's identity")),
    responses(
        (status = 200, description = "The post", body = Posts),
        (status = 404, description = "No published post has this identity", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_post(
    State(store): State<Store>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Posts>, Problem> {
    one(&store, &PostKey::Id(id), uri.path()).await
}

/// Reads one published post by its slug.
#[utoipa::path(
    get, path = "/posts/slug/{slug}/", tag = "posts", security(("content_key" = []), ("content_key_query" = [])),
    params(("slug" = String, Path, description = "The post's slug")),
    responses(
        (status = 200, description = "The post", body = Posts),
        (status = 404, description = "No published post has this slug", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_post_by_slug(
    State(store): State<Store>,
    Path(slug): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Posts>, Problem> {
    let slug = Slug::try_new(&slug).map_err(|_| not_found(uri.path()))?;
    one(&store, &PostKey::Slug(slug), uri.path()).await
}

async fn one(store: &Store, key: &PostKey, path: &str) -> Result<Json<Posts>, Problem> {
    let post = store
        .post(key)
        .await
        .map_err(|error| Problem::internal(&error))?;
    let post = post.ok_or_else(|| not_found(path))?;
    Ok(Json(Posts {
        posts: vec![post],
        meta: None,
    }))
}

/// Refuses fields a post does not have in `filter` and `order`.
fn check_fields(query: &ListQuery) -> Result<(), Problem> {
    let unknown = |parameter: &str, field: &str| {
        let message = UserMessage::new(
            format!("The {parameter} uses {field}, which is not a field here."),
            "The list cannot be sent until it is fixed.",
            format!(
                "Use one of these fields: {}.",
                Post::FILTER_FIELDS.join(", ")
            ),
        );
        Problem::new(ProblemType::InvalidQuery, &message).with_rule(parameter)
    };
    if let Some(filter) = query.filter() {
        filter
            .check_fields(&Post::FILTER_FIELDS)
            .map_err(|error| unknown("filter", error.field()))?;
    }
    match query
        .order()
        .iter()
        .find(|key| !Post::FILTER_FIELDS.contains(&key.field()))
    {
        Some(key) => Err(unknown("order", key.field())),
        None => Ok(()),
    }
}

fn not_found(path: &str) -> Problem {
    let message = UserMessage::new(
        "No published post is at this address.",
        "It may still be a draft, or its slug may have changed.",
        "Check the address, or list the posts at /api/content/v1/posts/.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
