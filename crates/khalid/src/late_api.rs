//! The Content API and the Admin API, built beside the first answer
//! instead of before it. Building them sets up the `OpenAPI` document, which
//! is most of what `khalid serve` did between rendering the home pages and
//! answering (docs/standards/performance.md, cold start), and no page
//! needs it. A request under `/api/` waits until they are built.

use std::convert::Infallible;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::sync::watch;
use tower::ServiceExt as _;
use tower::service_fn;

/// Where the APIs answer. Every route of `khalid_api` sits under it.
const UNDER: &str = "/api/";

/// `app`, with every request under `/api/` sent instead to the router
/// `build` makes, which runs on a blocking thread from now on (RULES.md
/// BL-RUN-7). Nothing is routed on the way, so the APIs see each request
/// as they would have in one merged router. Must be called inside the
/// tokio runtime.
pub(crate) fn router(app: Router, build: impl FnOnce() -> Router + Send + 'static) -> Router {
    let (tell, ready) = watch::channel(None);
    tokio::task::spawn_blocking(move || {
        // Nobody may be waiting yet, so a send with no receiver is fine.
        let _ = tell.send(Some(build()));
    });
    let answer = service_fn(move |request: Request<Body>| {
        let (app, ready) = (app.clone(), ready.clone());
        async move {
            if request.uri().path().starts_with(UNDER) {
                apis(ready, request).await
            } else {
                app.oneshot(request).await
            }
        }
    });
    Router::new().fallback_service(answer)
}

/// The built APIs' answer to `request`, once they are built.
async fn apis(
    mut ready: watch::Receiver<Option<Router>>,
    request: Request<Body>,
) -> Result<Response, Infallible> {
    let built = ready
        .wait_for(Option::is_some)
        .await
        .ok()
        .and_then(|built| built.clone());
    match built {
        Some(router) => router.oneshot(request).await,
        // The build panicked, so the APIs cannot answer at all.
        None => Ok(StatusCode::SERVICE_UNAVAILABLE.into_response()),
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::Path;
    use axum::routing::get;

    use super::*;

    fn site() -> Router {
        Router::new().fallback(|| async { StatusCode::IM_A_TEAPOT })
    }

    async fn status(app: Router, address: &str) -> StatusCode {
        let request = Request::get(address).body(Body::empty()).unwrap();
        app.oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn a_request_waits_for_the_built_apis_and_reaches_them_with_its_own_path() {
        let app = router(site(), || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let one = |Path(id): Path<String>| async move { id };
            Router::new().route("/api/admin/v1/posts/{id}/", get(one))
        });
        let answered = status(app, "/api/admin/v1/posts/why-links/").await;
        assert_eq!(answered, StatusCode::OK);
    }

    #[tokio::test]
    async fn every_other_address_is_the_sites() {
        let app = router(site(), Router::new);
        assert_eq!(status(app, "/blog/").await, StatusCode::IM_A_TEAPOT);
    }

    #[tokio::test]
    async fn a_build_that_fails_answers_unavailable() {
        let app = router(site(), || panic!("the build failed"));
        let answered = status(app, "/api/content/v1/posts/").await;
        assert_eq!(answered, StatusCode::SERVICE_UNAVAILABLE);
    }
}
