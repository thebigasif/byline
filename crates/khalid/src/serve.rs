//! `khalid serve`: the site answered over HTTP from rendered pages, shown
//! again after every saved edit, until Ctrl+C or a stop signal.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use khalid_content::EMBED_PROVIDERS;
use khalid_core::Config;
use khalid_routes::Routes;
use khalid_server::{LivePages, Pages, SecurityHeaders, router, watch};
use serde::Serialize;
use tokio::net::TcpListener;

use crate::cli::Output;
use crate::exit::Exit;
use crate::site_pages::SitePages;
use crate::{report, serve_message};

/// How often the site folder is checked for a saved edit (ADR "the server
/// watches files by polling").
const WATCH_EVERY: Duration = Duration::from_millis(500);

/// How long requests in flight may take to finish after a stop signal
/// before their connections are closed (RULES.md BL-RUN-10).
const GRACE: Duration = Duration::from_secs(10);

/// The JSON shape of the line `khalid serve --json` prints once it listens.
#[derive(Serialize)]
struct ServeBody<'a> {
    address: &'a str,
}

/// Serves the `site` folder, whose `config` and `routes` were checked, on `listen`.
pub(crate) fn run(
    site: &Path,
    config: &Config,
    routes: &Routes,
    listen: SocketAddr,
    output: Output,
) -> Exit {
    let started = SitePages::new(site, config.clone(), routes.clone()).and_then(|source| {
        let (pages, not_found) = source.render_all()?;
        let pages = Pages::new(pages, not_found).map_err(|_| serve_message::internal())?;
        Ok((source, pages))
    });
    let (source, pages) = match started {
        Ok(started) => started,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let origins: Vec<&str> = EMBED_PROVIDERS.iter().map(|p| p.frame_origin()).collect();
    let (Ok(security), Ok(runtime)) = (
        SecurityHeaders::new(&origins),
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build(),
    ) else {
        return report::failure(output, &serve_message::internal(), Exit::Internal);
    };
    let served = runtime.block_on(async {
        let listener = TcpListener::bind(listen)
            .await
            .map_err(|error| (serve_message::listen(listen, &error), Exit::Failure))?;
        let local = listener
            .local_addr()
            .map_err(|_| (serve_message::internal(), Exit::Internal))?;
        announce(output, config, &format!("http://{local}{}", config.mount()));
        let live = LivePages::new(pages);
        let watching = watch(site, &source, &live, WATCH_EVERY);
        let stopped = khalid_server::serve(listener, router(live, security), stop(), GRACE).await;
        watching.stop().await;
        stopped.map_err(|_| (serve_message::stopped(), Exit::Failure))
    });
    match served {
        Ok(_) if output == Output::Human => report::text("Stopped.\n", Exit::Success),
        Ok(_) => Exit::Success,
        Err((message, exit)) => report::failure(output, &message, exit),
    }
}

/// Says where the site answers. In JSON, this one line is the whole output.
fn announce(output: Output, config: &Config, address: &str) {
    match output {
        Output::Human => report::text(
            &format!(
                "Serving {} at {address}. Press Ctrl+C to stop.\n",
                config.name()
            ),
            Exit::Success,
        ),
        Output::Json => report::json(&ServeBody { address }, Exit::Success),
    };
}

/// Completes on Ctrl+C or, on Unix, the stop signal a service manager sends.
async fn stop() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut terminate) = signal(SignalKind::terminate()) {
            // A Ctrl+C listener that fails is left out, rather than taken
            // for a stop.
            tokio::select! {
                Ok(()) = tokio::signal::ctrl_c() => {}
                _ = terminate.recv() => {}
            }
            return;
        }
    }
    if tokio::signal::ctrl_c().await.is_err() {
        tracing::warn!("Ctrl+C cannot be heard here, so stop the server with its process id");
        std::future::pending::<()>().await;
    }
}
