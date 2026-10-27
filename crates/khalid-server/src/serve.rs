//! Serving until told to stop, then stopping gracefully.

use std::fmt::Debug;
use std::future::{Future, IntoFuture, pending};
use std::time::Duration;

use axum::Router;
use axum::serve::{Listener, ListenerExt};
use tokio::sync::oneshot;

use crate::ServerError;

/// How the server stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// Every request in flight was answered before the server stopped.
    Finished,
    /// Some connections were still open when the grace period ran out, and
    /// were closed unanswered.
    GraceExpired,
}

/// Answers requests on `listener` with `router` until `shutdown` completes,
/// then stops gracefully (RULES.md BL-RUN-10).
///
/// The listener is a plain `tokio::net::TcpListener`, or one that wraps each
/// connection, such as the TLS listener `khalid serve --domain` uses.
///
/// When `shutdown` completes, the server stops accepting connections and
/// finishes the requests it has. Connections still open after `grace` are
/// closed. The caller then flushes its queues and closes the database.
///
/// # Errors
///
/// Returns [`ServerError::Serve`] when the listener fails.
pub async fn serve<L, F>(
    listener: L,
    router: Router,
    shutdown: F,
    grace: Duration,
) -> Result<Stopped, ServerError>
where
    L: Listener,
    L::Addr: Clone + Debug + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let (signalled, on_signal) = oneshot::channel::<()>();
    let signal = async move {
        shutdown.await;
        tracing::info!("stopping: no new connections, finishing the requests in flight");
        let _ = signalled.send(());
    };
    // Handlers that limit by address, such as the comment form, read the
    // peer through `ConnectInfo` of the listener's address, a `SocketAddr`
    // for TCP and TLS. Tapping no connection is what gives any listener it.
    let listener = listener.tap_io(|_| {});
    let router = router.into_make_service_with_connect_info::<L::Addr>();
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(signal)
        .into_future();
    let deadline = async move {
        match on_signal.await {
            Ok(()) => tokio::time::sleep(grace).await,
            // The signal future is gone only once the server has finished.
            Err(_) => pending().await,
        }
    };
    tokio::select! {
        result = server => {
            result.map_err(|source| ServerError::Serve { source })?;
            Ok(Stopped::Finished)
        }
        () = deadline => {
            tracing::warn!(?grace, "stopped with connections still open after the grace period");
            Ok(Stopped::GraceExpired)
        }
    }
}
