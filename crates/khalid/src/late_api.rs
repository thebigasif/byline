//! The Content API and the Admin API, built when the first request under
//! `/api/` arrives instead of at start. Building them sets up the `OpenAPI`
//! document, which took most of the time between rendering the home pages
//! and answering (docs/standards/performance.md, cold start), and the
//! router and its document are the largest thing an idle server would
//! hold for them. No page needs either. A request under `/api/` waits
//! until they are built.

use std::convert::Infallible;
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::sync::watch;
use tower::ServiceExt as _;
use tower::service_fn;

/// Where the APIs answer. Every route of `khalid_api` sits under it.
const UNDER: &str = "/api/";

/// What makes the APIs' router.
type Build = Box<dyn FnOnce() -> Router + Send>;

/// The APIs, built once, on the first request that needs them.
struct Late {
    /// The build and where it says it is done, until a request takes them.
    build: Mutex<Option<(Build, watch::Sender<Option<Router>>)>>,
    /// The built router, once there is one.
    ready: watch::Receiver<Option<Router>>,
}

impl Late {
    /// Starts the build on a blocking thread (RULES.md BL-RUN-7), unless a
    /// request already started it. The build finishes even when the request
    /// that started it goes away, so the next one finds it built.
    fn start(&self) {
        let taken = self
            .build
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some((build, tell)) = taken {
            tokio::task::spawn_blocking(move || {
                // `Late` keeps a receiver, so the send always lands.
                let _ = tell.send(Some(build()));
            });
        }
    }
}

/// `app`, with every request under `/api/` sent instead to the router
/// `build` makes, built when the first of them arrives. Nothing is routed on
/// the way, so the APIs see each request as they would have in one merged
/// router.
pub(crate) fn router(app: Router, build: impl FnOnce() -> Router + Send + 'static) -> Router {
    let (tell, ready) = watch::channel(None);
    let build: Build = Box::new(build);
    let late = Arc::new(Late {
        build: Mutex::new(Some((build, tell))),
        ready,
    });
    let answer = service_fn(move |request: Request<Body>| {
        let (app, late) = (app.clone(), Arc::clone(&late));
        async move {
            if request.uri().path().starts_with(UNDER) {
                apis(&late, request).await
            } else {
                app.oneshot(request).await
            }
        }
    });
    Router::new().fallback_service(answer)
}

/// The built APIs' answer to `request`, once they are built.
async fn apis(late: &Late, request: Request<Body>) -> Result<Response, Infallible> {
    late.start();
    let mut ready = late.ready.clone();
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    /// A build of an empty router that counts how often it ran.
    fn counted() -> (Arc<AtomicUsize>, impl FnOnce() -> Router + Send + 'static) {
        let builds = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&builds);
        let build = move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Router::new()
        };
        (builds, build)
    }

    #[tokio::test]
    async fn the_apis_are_not_built_until_a_request_under_api_arrives() {
        let (builds, build) = counted();
        let app = router(site(), build);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(builds.load(Ordering::SeqCst), 0, "not at start");
        status(app.clone(), "/blog/").await;
        assert_eq!(builds.load(Ordering::SeqCst), 0, "not for a page");
        status(app, "/api/content/v1/posts/").await;
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn requests_at_once_build_the_apis_once() {
        let (builds, build) = counted();
        let app = router(site(), build);
        let address = "/api/content/v1/posts/";
        tokio::join!(
            status(app.clone(), address),
            status(app.clone(), address),
            status(app.clone(), address),
        );
        status(app, address).await;
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_build_that_fails_answers_unavailable() {
        let app = router(site(), || panic!("the build failed"));
        let answered = status(app, "/api/content/v1/posts/").await;
        assert_eq!(answered, StatusCode::SERVICE_UNAVAILABLE);
    }
}
