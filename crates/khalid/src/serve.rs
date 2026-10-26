//! `khalid serve`: the site answered over HTTP from rendered pages, the
//! Content API from the site's own index, and the Admin API over its files,
//! each showing every saved edit, published post or API write again after
//! a moment, until Ctrl+C or a stop signal.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use khalid_api::api;
use khalid_content::{EMBED_PROVIDERS, SiteContent};
use khalid_core::{Config, Slug, SystemClock, UserMessage};
use khalid_db::{Database, KeyTable, SiteId};
use khalid_routes::Routes;
use khalid_server::{FolderLook, LivePages, Pages, SecurityHeaders, router, watch};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

use crate::cli::Output;
use crate::exit::Exit;
use crate::site_content::LiveContent;
use crate::site_pages::{ContentFeed, SitePages};
use crate::site_posts::SitePosts;
use crate::{db, db_message, report, serve_message};

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

/// Serves the `site` folder, whose `config`, `routes` and content were
/// checked, on `listen`. The database opens first and closes last, after
/// every request and the watch have stopped.
///
/// `start` holds the checked content and a look at the folder taken before
/// it was read. The first pages are rendered from that content, which is
/// then dropped, so the folder is read once at start and a file saved from
/// the look on is always seen as an edit.
pub(crate) fn run(
    site: &Path,
    (config, routes): (&Config, &Routes),
    start: (SiteContent, FolderLook),
    listen: SocketAddr,
    output: Output,
) -> Exit {
    let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    else {
        return report::failure(output, &serve_message::internal(), Exit::Internal);
    };
    let (database, id) = match open(site, &runtime) {
        Ok(opened) => opened,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let served = serve_with(
        (site, config, routes),
        start,
        (listen, output),
        &runtime,
        (&database, id),
    );
    runtime.block_on(database.close());
    match served {
        Ok(()) if output == Output::Human => report::text("Stopped.\n", Exit::Success),
        Ok(()) => Exit::Success,
        Err((message, exit)) => report::failure(output, &message, exit),
    }
}

/// Renders the site with its checked settings and routes, then answers on
/// `listen` until told to stop.
fn serve_with(
    (site, config, routes): (&Path, &Config, &Routes),
    (checked, look): (SiteContent, FolderLook),
    (listen, output): (SocketAddr, Output),
    runtime: &Runtime,
    (database, id): (&Database, SiteId),
) -> Result<(), (UserMessage, Exit)> {
    let content = LiveContent::default();
    let feed = ContentFeed {
        runtime: runtime.handle().clone(),
        db: database.clone(),
        site: id,
        live: content.clone(),
    };
    let (source, pages) = SitePages::new(site, config.clone(), routes.clone(), Some(feed))
        .and_then(|source| {
            let (pages, not_found) = source.render_content(&checked)?;
            drop(checked);
            // Brotli copies are added once the server listens, so the first
            // reader does not wait for them (docs/standards/performance.md).
            let pages = Pages::plain(pages, not_found).map_err(|_| serve_message::internal())?;
            Ok((source, pages))
        })
        .map_err(|message| (message, Exit::Failure))?;
    let origins: Vec<&str> = EMBED_PROVIDERS.iter().map(|p| p.frame_origin()).collect();
    let security =
        SecurityHeaders::new(&origins).map_err(|_| (serve_message::internal(), Exit::Internal))?;
    let keys = KeyTable::new(database.clone(), id, SystemClock);
    runtime.block_on(async {
        let listener = TcpListener::bind(listen)
            .await
            .map_err(|error| (serve_message::listen(listen, &error), Exit::Failure))?;
        let local = listener
            .local_addr()
            .map_err(|_| (serve_message::internal(), Exit::Internal))?;
        announce(output, config, &format!("http://{local}{}", config.mount()));
        let live = LivePages::new(pages);
        let compressing = live.clone();
        tokio::spawn(async move {
            if let Err(error) = compressing.compress().await {
                tracing::warn!(
                    ?error,
                    "pages could not be compressed, so they are sent plain"
                );
            }
        });
        let watching = watch(site, &source, &live, look, WATCH_EVERY);
        let posts = SitePosts::new(site, config.clone(), SystemClock);
        let (api, _) = api::router(Arc::new(content), Arc::new(posts), Arc::new(keys));
        let app = api.merge(router(live, security));
        let stopped = khalid_server::serve(listener, app, stop(), GRACE).await;
        watching.stop().await;
        stopped
            .map(|_| ())
            .map_err(|_| (serve_message::stopped(), Exit::Failure))
    })
}

/// Opens `data/site.db` in the site folder, making it when it is missing,
/// brings it up to date, and finds the site's row.
fn open(site: &Path, runtime: &Runtime) -> Result<(Database, SiteId), UserMessage> {
    let path = db::data_file(site)?;
    let main = Slug::try_new(db::MAIN).map_err(|_| db_message::internal())?;
    runtime.block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|error| db_message::of(&path, &error))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        }
        .await;
        match opened {
            Ok(id) => Ok((database, id)),
            Err(error) => {
                database.close().await;
                Err(db_message::of(&path, &error))
            }
        }
    })
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
