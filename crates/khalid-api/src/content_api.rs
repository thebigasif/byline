//! The Content API under `/api/content/v1/`.

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, Query, State};
use axum::middleware;
use axum::{Json, Router};
use khalid_core::{Slug, UserMessage};
use serde::Serialize;
use utoipa::openapi::OpenApi as Document;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::{ApiDoc, finish};
use crate::api_version::negotiate_version;
use crate::content_key::require_content_key;
use crate::content_store::{ContentStore, PostKey};
use crate::etag::tag_and_revalidate;
use crate::keys::KeyStore;
use crate::pagination::Meta;
use crate::problem::{Problem, ProblemType};
use crate::query::{ListQuery, RawListQuery};
use crate::resources::{Collection, Post};

/// Where the Content API lives.
pub(crate) const BASE: &str = "/api/content/v1";

/// The store every route reads from.
type Store = Arc<dyn ContentStore>;

/// The Content API alone and `/api/openapi.json`. [`api::router`] serves
/// every API behind one document.
///
/// [`api::router`]: crate::api::router
pub fn router(store: Store, keys: Arc<dyn KeyStore>) -> (Router, Document) {
    finish(OpenApiRouter::with_openapi(ApiDoc::openapi()).nest(BASE, routes(store, keys)))
}

/// The Content API routes, each declared with `utoipa::path` and
/// registered through `routes!`, so a route cannot exist without its
/// `OpenAPI` entry (RULES.md BL-API-4). Every route needs a content key.
pub(crate) fn routes(store: Store, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(browse_posts))
        .routes(routes!(read_post))
        .routes(routes!(read_post_by_slug))
        .routes(routes!(browse_pages))
        .routes(routes!(read_page))
        .routes(routes!(read_page_by_slug))
        .routes(routes!(browse_glossary))
        .routes(routes!(read_glossary_term))
        .routes(routes!(read_glossary_term_by_slug))
        .routes(routes!(browse_case_studies))
        .routes(routes!(read_case_study))
        .routes(routes!(read_case_study_by_slug))
        .routes(routes!(browse_changelog))
        .routes(routes!(read_changelog_entry))
        .routes(routes!(read_changelog_entry_by_slug))
        .route_layer(middleware::from_fn_with_state(keys, require_content_key))
        .route_layer(middleware::from_fn(tag_and_revalidate))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(store)
}

/// Declares one collection's envelope and its three routes: the list, one
/// entry by identity and one by slug. Each route keeps its own `OpenAPI`
/// entry and operation id; the work is in [`browse`] and [`one`].
macro_rules! collection_routes {
    (
        $collection:expr, $Envelope:ident, $key:ident, $tag:tt,
        $browse:ident = $list_path:tt,
        $read:ident = $id_path:tt,
        $read_by_slug:ident = $slug_path:tt $(,)?
    ) => {
        #[doc = concat!("Entries of `", $tag, "`, as every response wraps them.")]
        #[derive(Debug, Serialize, ToSchema)]
        pub struct $Envelope {
            /// The entries on this page, or the one entry asked for.
            $key: Vec<Post>,
            /// Pagination, on lists only.
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: Option<Meta>,
        }

        #[doc = concat!("Lists the published entries of `", $tag, "`.")]
        #[utoipa::path(
            get, path = $list_path, tag = $tag, params(RawListQuery),
            security(("content_key" = []), ("content_key_query" = [])),
            responses(
                (status = 200, description = "One page of published entries", body = $Envelope),
                (status = 400, description = "A list parameter could not be read", body = Problem, content_type = "application/problem+json"),
                (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
            )
        )]
        async fn $browse(
            State(store): State<Store>,
            Query(raw): Query<RawListQuery>,
        ) -> Result<Json<$Envelope>, Problem> {
            let (entries, meta) = browse(&store, $collection, &raw).await?;
            Ok(Json($Envelope { $key: entries, meta: Some(meta) }))
        }

        #[doc = concat!("Reads one published entry of `", $tag, "` by its identity.")]
        #[utoipa::path(
            get, path = $id_path, tag = $tag,
            security(("content_key" = []), ("content_key_query" = [])),
            params(("id" = String, Path, description = "The entry's identity")),
            responses(
                (status = 200, description = "The entry", body = $Envelope),
                (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
                (status = 404, description = "No published entry has this identity", body = Problem, content_type = "application/problem+json"),
            )
        )]
        async fn $read(
            State(store): State<Store>,
            Path(id): Path<String>,
            OriginalUri(uri): OriginalUri,
        ) -> Result<Json<$Envelope>, Problem> {
            let entry = one(&store, $collection, &PostKey::Id(id), uri.path()).await?;
            Ok(Json($Envelope { $key: vec![entry], meta: None }))
        }

        #[doc = concat!("Reads one published entry of `", $tag, "` by its slug.")]
        #[utoipa::path(
            get, path = $slug_path, tag = $tag,
            security(("content_key" = []), ("content_key_query" = [])),
            params(("slug" = String, Path, description = "The entry's slug")),
            responses(
                (status = 200, description = "The entry", body = $Envelope),
                (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
                (status = 404, description = "No published entry has this slug", body = Problem, content_type = "application/problem+json"),
            )
        )]
        async fn $read_by_slug(
            State(store): State<Store>,
            Path(slug): Path<String>,
            OriginalUri(uri): OriginalUri,
        ) -> Result<Json<$Envelope>, Problem> {
            let slug = Slug::try_new(&slug).map_err(|_| not_found(uri.path()))?;
            let entry = one(&store, $collection, &PostKey::Slug(slug), uri.path()).await?;
            Ok(Json($Envelope { $key: vec![entry], meta: None }))
        }
    };
}

collection_routes!(
    Collection::Posts,
    Posts,
    posts,
    "posts",
    browse_posts = "/posts/",
    read_post = "/posts/{id}/",
    read_post_by_slug = "/posts/slug/{slug}/",
);
collection_routes!(
    Collection::Pages,
    Pages,
    pages,
    "pages",
    browse_pages = "/pages/",
    read_page = "/pages/{id}/",
    read_page_by_slug = "/pages/slug/{slug}/",
);
collection_routes!(
    Collection::Glossary,
    GlossaryTerms,
    glossary,
    "glossary",
    browse_glossary = "/glossary/",
    read_glossary_term = "/glossary/{id}/",
    read_glossary_term_by_slug = "/glossary/slug/{slug}/",
);
collection_routes!(
    Collection::CaseStudies,
    CaseStudies,
    case_studies,
    "case-studies",
    browse_case_studies = "/case-studies/",
    read_case_study = "/case-studies/{id}/",
    read_case_study_by_slug = "/case-studies/slug/{slug}/",
);
collection_routes!(
    Collection::Changelog,
    ChangelogEntries,
    changelog,
    "changelog",
    browse_changelog = "/changelog/",
    read_changelog_entry = "/changelog/{id}/",
    read_changelog_entry_by_slug = "/changelog/slug/{slug}/",
);

/// One page of `collection` for a list request, with its pagination.
async fn browse(
    store: &Store,
    collection: Collection,
    raw: &RawListQuery,
) -> Result<(Vec<Post>, Meta), Problem> {
    let query = ListQuery::try_from(raw)?;
    check_fields(&query)?;
    let listed = store
        .list(collection, &query)
        .await
        .map_err(|error| Problem::internal(&error))?;
    let meta = Meta::new(query.page(), query.limit(), listed.total());
    let formats = query.formats();
    let entries = listed
        .into_items()
        .into_iter()
        .map(|entry| entry.with_formats(formats));
    Ok((entries.collect(), meta))
}

/// One entry of `collection`, or a 404 problem about `path`.
async fn one(
    store: &Store,
    collection: Collection,
    key: &PostKey,
    path: &str,
) -> Result<Post, Problem> {
    let entry = store
        .find(collection, key)
        .await
        .map_err(|error| Problem::internal(&error))?;
    entry.ok_or_else(|| not_found(path))
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
        "Nothing published is at this address.",
        "It may still be a draft, or its slug may have changed.",
        "Check the address, or fetch the list it belongs to, such as /api/content/v1/posts/.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
