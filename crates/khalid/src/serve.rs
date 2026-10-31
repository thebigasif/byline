//! `khalid serve`: the site answered over HTTP from rendered pages with its
//! sitemaps, feeds and moved addresses, the Content API from the site's own
//! index, and the Admin API over its files, each showing every saved edit,
//! published post or API write again after a moment, until Ctrl+C or a stop
//! signal. A headless site has no pages, and every page address answers 404.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axum::Router;
use khalid_api::api::{self, AdminDesks};
use khalid_api::comment_moderation::CommentModeration;
use khalid_api::media_api::MediaLibrary;
use khalid_api::publish_door::PublishDoor;
use khalid_api::redirect_desk::RedirectDesk;
use khalid_api::resources::AdminPage;
use khalid_api::session_check::SessionCheck;
use khalid_api::unlock::{Transport, UnlockSigner};
use khalid_content::{EMBED_PROVIDERS, SiteContent};
use khalid_core::{Config, Environment, S3Keys, Secret, Slug, SystemClock, UserMessage};
use khalid_db::{Database, KeyTable, ScheduleTable, SiteId, WebhookTable};
use khalid_outbound::{Outbound, OutboundError};
use khalid_routes::Routes;
use khalid_server::{FolderLook, LivePages, Pages, SecurityHeaders, router, watch};
use serde::Serialize;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use crate::announce::SiteAnnounce;
use crate::cli::Output;
use crate::content_feed::{self, Destination};
use crate::exit::Exit;
use crate::memory::HandBack;
use crate::post_files::Assets;
use crate::serve_https::{self, Listen};
use crate::site_content::LiveContent;
use crate::site_entries::SiteEntries;
use crate::site_pages::SitePages;
use crate::site_posts::SitePosts;
use crate::{
    announce, comment_pages, db, db_message, media_library, newsletter, preview, redirect_route,
    report, scheduler, search_route, serve_message, share_images, staff, unlock,
};

/// How often the site folder is looked at for a saved edit where the system
/// has no file events for it (ADR "the server watches files with events").
const WATCH_EVERY: Duration = Duration::from_millis(500);

/// How long requests in flight may take to finish after a stop signal
/// before their connections are closed (RULES.md BL-RUN-10).
const GRACE: Duration = Duration::from_secs(10);

/// The JSON shape of the line `khalid serve --json` prints once it listens.
#[derive(Serialize)]
struct ServeBody<'a> {
    address: &'a str,
}

/// The secrets `khalid serve` reads from the environment at start.
#[derive(Debug)]
pub(crate) struct Keys {
    /// The media bucket's two keys, when the environment has them.
    pub(crate) bucket: Option<S3Keys>,
    /// The password, key or token the `[mail]` transport signs in with.
    pub(crate) mail: Option<Secret<String>>,
}

impl Keys {
    /// The keys `env` holds for the site with `config`.
    pub(crate) fn read(env: &Environment, config: &Config) -> Self {
        let mail = config.mail().map(|mail| &mail.transport);
        Self {
            bucket: env.s3_keys(),
            mail: mail.and_then(|transport| env.mail_secret(transport)),
        }
    }
}

/// Serves the `site` folder, whose `config`, `routes` and content were
/// checked, on `listen`. The database opens first and closes last, after
/// every request, the watch, the scheduler and the announcing have stopped. Webhooks and
/// `IndexNow` are sent through `outbound`, the one shared client.
///
/// `start` holds the checked content, the files beside its folder posts,
/// and a look at the folder taken before it was read. The first pages are
/// rendered from that content, which is then dropped, so the folder is read
/// once at start and a file saved from the look on is always seen as an
/// edit.
pub(crate) fn run(
    site: &Path,
    (config, routes): (&Config, &Routes),
    start: (SiteContent, Assets, FolderLook),
    (listen, outbound, keys): (&Listen, Result<Outbound, OutboundError>, Keys),
    output: Output,
) -> Exit {
    let built = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_keep_alive(Duration::from_secs(1))
        .build();
    let (Ok(runtime), Ok(outbound)) = (built, outbound) else {
        return report::failure(output, &serve_message::internal(), Exit::Internal);
    };
    let (database, id) = match open(site, &runtime) {
        Ok(opened) => opened,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let served = serve_with(
        (site, config, routes),
        start,
        (listen, &outbound, output, keys),
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
    (checked, assets, look): (SiteContent, Assets, FolderLook),
    (listen, outbound, output, keys): (&Listen, &Outbound, Output, Keys),
    runtime: &Runtime,
    (database, id): (&Database, SiteId),
) -> Result<(), (UserMessage, Exit)> {
    let (content, hand_back) = (LiveContent::default(), HandBack::default());
    let (redirects, addresses) = redirect_route::stores((database, id));
    let to = destination(
        site,
        config,
        (database, id),
        (&content, &hand_back),
        (&redirects, &addresses),
    );
    let (feed, feeding) = content_feed::start(runtime.handle(), to);
    // Only the home pages are rendered before the server listens; the rest
    // follows behind the first answer (ADR "khalid serve answers before the
    // full render").
    let (source, first) = SitePages::new(site, config.clone(), routes.clone(), Some(feed))
        .and_then(|source| {
            let first = source.render_first(&checked)?;
            let first = Pages::first(first).map_err(|_| serve_message::internal())?;
            let first = match source.stylesheet() {
                Some(href) => first.with_preloads(vec![href]),
                None => first,
            };
            Ok((source, first))
        })
        .map_err(|message| (message, Exit::Failure))?;
    let security = security_headers()?;
    let search = search_route::Search {
        pages: source.clone(),
        db: database.clone(),
        site: id,
        security: security.clone(),
    };
    let newsletters = newsletter::router(config, (database, id), (keys.mail, outbound))
        .map_err(|message| (message, Exit::Internal))?;
    let served = runtime.block_on(async {
        let (listener, address) = serve_https::open(site, listen).await?;
        announce(output, config, &format!("{address}{}", config.mount()));
        let live = LivePages::new(first);
        hand_back.start(live.replacements());
        let (failed, failure) = oneshot::channel();
        let rest = (source.clone(), live.clone());
        tokio::spawn(render_the_rest(rest, (checked, assets), failed));
        let watching = watch(site, &source, &live, look, WATCH_EVERY);
        // Each part reads the database or the folder alone, so all open at once.
        let (telling, announcing) = announce::start(config, (database, id), outbound)
            .map_err(|message| (message, Exit::Failure))?;
        let replies = (security.clone(), telling.clone());
        let commenting = comment_pages::start(config, (database, id), (&source, &live), replies);
        let shown_pages = router(live.clone(), security.clone());
        let ((comments, moderation), door, (media, files), unlocked) = tokio::try_join!(
            async { Ok(commenting.await) },
            publish_door((site, config), (database, id), telling),
            media_library::open(site, config, keys.bucket, outbound),
            unlocking((site, config), (database, id), &source, shown_pages),
        )?;
        let scheduling = scheduler::start(Arc::clone(&door));
        let previews = preview::router(Arc::clone(&door), source.clone(), config, security.clone());
        let pages = (unlocked, live, config.mount());
        let (desk, pages) =
            redirect_route::start((database, id), pages, (redirects, addresses)).await;
        let desks = (moderation, media, desk);
        let apis = api_router((content, door), desks, (site, database, id, config));
        let app = assembled(
            (site, config, &security),
            (apis, files, previews, search_route::router(config, search)),
            (comments, newsletters, pages),
        );
        let why = Arc::new(Mutex::new(None));
        let stopping = stop_or_fail(failure, Arc::clone(&why));
        let stopped = khalid_server::serve(listener, app, stopping, GRACE).await;
        watching.stop().await;
        scheduling.stop().await;
        announcing.stop().await;
        if let Some(message) = why.lock().unwrap_or_else(PoisonError::into_inner).take() {
            return Err((message, Exit::Failure));
        }
        stopped
            .map(|_| ())
            .map_err(|_| (serve_message::stopped(), Exit::Failure))
    });
    // The pages hold the last feed. Once they are gone the Content API's
    // task finishes its last write, before the database closes.
    drop(source);
    if let Err(error) = runtime.block_on(feeding) {
        tracing::warn!(?error, "the Content API's task stopped early");
    }
    served
}

/// The Content API and the Admin API as `khalid serve` runs them, with the
/// comment moderation, media library, stored redirects, webhooks and staff
/// accounts of site `id`.
fn api_router(
    (content, door): (LiveContent, Arc<PublishDoor>),
    (comments, media, redirects): (Arc<dyn CommentModeration>, MediaLibrary, RedirectDesk),
    (site, database, id, config): (&Path, &Database, SiteId, &Config),
) -> Router {
    let accounts = staff::accounts(database, id, config);
    let sessions: Arc<dyn SessionCheck> = Arc::new(accounts.clone());
    let staff = (khalid_auth::staff_routes(accounts), sessions);
    let desks = AdminDesks {
        comments,
        media: Some(media),
        redirects,
        webhooks: Some(Arc::new(WebhookTable::new(
            database.clone(),
            id,
            SystemClock,
        ))),
        pages: Some(Arc::new(SiteEntries::<AdminPage>::new(site, config))),
    };
    let keys = Arc::new(KeyTable::new(database.clone(), id, SystemClock));
    let (api, _) = api::router_for_serve(Arc::new(content), door, keys, desks, staff);
    api
}

/// The pages, with the stylesheet named as a preload on every HTML answer
/// when the site has one (G20).
fn with_hint(pages: Pages, source: &SitePages) -> Pages {
    match source.stylesheet() {
        Some(href) => pages.with_preloads(vec![href]),
        None => pages,
    }
}

/// Where the Content API's feed writes, as `khalid serve` sets it up.
fn destination(
    site: &Path,
    config: &Config,
    (database, id): (&Database, SiteId),
    (live, hand_back): (&LiveContent, &HandBack),
    (redirects, addresses): (
        &crate::live_redirects::LiveRedirects,
        &crate::site_addresses::LiveAddresses,
    ),
) -> Destination {
    Destination {
        folder: site.to_path_buf(),
        config: config.clone(),
        db: database.clone(),
        site: id,
        live: live.clone(),
        hand_back: hand_back.clone(),
        addresses: addresses.clone(),
        redirects: redirects.clone(),
    }
}

/// The one router the server answers: the APIs, the site's own routes, the
/// drawn share cards, the admin, the comments and the newsletters.
fn assembled(
    (site, config, security): (&Path, &Config, &SecurityHeaders),
    (api, files, previews, searches): (Router, Router, Router, Router),
    (comments, newsletters, pages): (Router, Router, Router),
) -> Router {
    api.merge(files)
        .merge(previews)
        .merge(searches)
        .merge(share_images::router(site, config, security.clone()))
        .merge(khalid_admin::router(khalid_admin::built()))
        .merge(comments)
        .merge(newsletters)
        .merge(pages)
}

/// The publish door the Admin API, the scheduler and preview links share.
/// It keeps schedules in the database and tells `telling` about each post
/// that goes live.
async fn publish_door(
    (site, config): (&Path, &Config),
    (database, id): (&Database, SiteId),
    telling: SiteAnnounce,
) -> Result<Arc<PublishDoor>, (UserMessage, Exit)> {
    let file = db::data_file(site).map_err(|message| (message, Exit::Failure))?;
    let posts = SitePosts::new(site, config.clone(), SystemClock);
    let schedules = Arc::new(ScheduleTable::new(database.clone(), id));
    let signer = preview::signer(config, (database, id, &file)).await;
    let door = PublishDoor::new(Arc::new(posts), Arc::new(telling))
        .with_schedules(schedules, SystemClock)
        .with_previews(signer.map_err(|message| (message, Exit::Failure))?);
    Ok(Arc::new(door))
}

/// `pages`, the site's page router, with the password form and the cookie
/// check of `source`'s password posts. The cookies are signed with the
/// site's preview key under their own label, and marked `Secure` when the
/// site's address is `https`.
async fn unlocking(
    (site, config): (&Path, &Config),
    (database, id): (&Database, SiteId),
    source: &SitePages,
    pages: axum::Router,
) -> Result<axum::Router, (UserMessage, Exit)> {
    let file = db::data_file(site).map_err(|message| (message, Exit::Failure))?;
    let key = database.preview_key(id, &SystemClock).await;
    let key = key.map_err(|error| (db_message::of(&file, &error), Exit::Failure))?;
    let transport = if config.address().as_str().starts_with("https://") {
        Transport::Https
    } else {
        Transport::Http
    };
    let signer = UnlockSigner::new(&key, transport, SystemClock);
    Ok(unlock::router(
        config,
        source.gate(),
        signer,
        pages,
        SystemClock,
    ))
}

/// Renders every page from the `checked` content on a blocking thread
/// (RULES.md BL-RUN-7), serves them in place of the first pages, then adds
/// their Brotli copies. A save during the start may have served a whole set
/// already, which is newer and stays. When the pages cannot be rendered,
/// `failed` gets why, and the server stops with it, as it did when it
/// rendered every page before listening.
async fn render_the_rest(
    (source, live): (SitePages, LivePages),
    (checked, assets): (SiteContent, Assets),
    failed: oneshot::Sender<UserMessage>,
) {
    let first = live.current();
    let rendering = tokio::task::spawn_blocking(move || {
        let served = source.render_content(&checked, assets)?;
        drop(checked);
        Pages::plain(served.pages, served.not_found)
            .map(|pages| pages.with_moves(served.moves))
            .map(|pages| with_hint(pages, &source))
            .map_err(|_| serve_message::internal())
    });
    let pages = match rendering.await {
        Ok(Ok(pages)) => pages,
        Ok(Err(message)) => {
            let _ = failed.send(message);
            return;
        }
        Err(_) => {
            let _ = failed.send(serve_message::internal());
            return;
        }
    };
    if live.replace_if_current(&first, pages)
        && let Err(error) = live.compress().await
    {
        tracing::warn!(
            ?error,
            "pages could not be compressed, so they are sent plain"
        );
    }
}

/// Completes on a stop signal, or once `failure` says why the site's pages
/// could not be rendered, which it leaves in `why`.
async fn stop_or_fail(
    failure: oneshot::Receiver<UserMessage>,
    why: Arc<Mutex<Option<UserMessage>>>,
) {
    tokio::select! {
        () = stop() => {}
        Ok(message) = failure => {
            *why.lock().unwrap_or_else(PoisonError::into_inner) = Some(message);
        }
    }
}

/// Opens `data/site.db` in the site folder, making it when it is missing,
/// brings it up to date, and finds the site's row.
pub(crate) fn open(site: &Path, runtime: &Runtime) -> Result<(Database, SiteId), UserMessage> {
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

/// The security headers of every page, letting in the frames of the embed
/// providers.
fn security_headers() -> Result<SecurityHeaders, (UserMessage, Exit)> {
    let origins: Vec<&str> = EMBED_PROVIDERS.iter().map(|p| p.frame_origin()).collect();
    SecurityHeaders::new(&origins).map_err(|_| (serve_message::internal(), Exit::Internal))
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
