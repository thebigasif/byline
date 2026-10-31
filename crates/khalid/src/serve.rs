//! `khalid serve`: the site answered over HTTP from rendered pages with its
//! sitemaps, feeds and moved addresses, the Content API from the site's own
//! index, and the Admin API over its files, each showing every saved edit,
//! published post or API write again after a moment, until Ctrl+C or a stop
//! signal. A headless site has no pages, and every page address answers 404.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axum::Router;
use khalid_analytics::VisitorLogStore;
use khalid_api::analytics_desk::AnalyticsDesk;
use khalid_api::api::{self, AdminDesks, AuditDesk};
use khalid_api::comment_moderation::CommentModeration;
use khalid_api::media_api::MediaLibrary;
use khalid_api::publish_door::PublishDoor;
use khalid_api::redirect_desk::RedirectDesk;
use khalid_api::resources::{AdminAuthor, AdminPage, AdminSnippet};
use khalid_api::session_check::SessionCheck;
use khalid_api::unlock::{Transport, UnlockSigner};
use khalid_auth::SiteDirectory;
use khalid_content::{EMBED_PROVIDERS, SiteContent};
use khalid_core::{Config, Environment, S3Keys, Secret, Slug, SystemClock, UserMessage};
use khalid_db::{
    AnalyticsTable, AuditTable, Database, KeyTable, NotFoundTable, ScheduleTable, SiteId,
    VisitorLogTable, WebhookTable,
};
use khalid_forms::FormDelivery;
use khalid_https::{Domain, HttpsFolder};
use khalid_newsletter::Mailer;
use khalid_outbound::{Outbound, OutboundError};
use khalid_routes::Routes;
use khalid_server::{FolderLook, LivePages, Pages, SecurityHeaders, watch};
use serde::Serialize;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use crate::alert_watch::Every;
use crate::announce::{Announcing, SiteAnnounce};
use crate::cli::Output;
use crate::content_feed::{self, Destination};
use crate::entry_events::Entries;
use crate::exit::Exit;
use crate::form_delivery::SiteDelivery;
use crate::memory::HandBack;
use crate::post_files::Assets;
use crate::serve_https::{self, Listen};
use crate::settings_events::Settings;
use crate::site_categories::SiteCategories;
use crate::site_content::LiveContent;
use crate::site_entries::SiteEntries;
use crate::site_hosts::{self, Among, Door};
use crate::site_pages::SitePages;
use crate::site_posts::SitePosts;
use crate::site_schema_templates::SiteSchemaTemplates;
use crate::site_settings::SiteSettings;
use crate::site_tags::SiteTags;
use crate::site_watch::SiteWatch;
use crate::tag_events::Tags;
use crate::{
    alert_watch, announce, assets, comment_pages, consent_route, db, db_message,
    draft_comment_mail, form_pages, media_library, newsletter, page_counts, preview, push_hook,
    redirect_route, report, scheduler, search_route, serve_message, share_images, staff,
    staff_toolbar, unlock,
};

/// How often the site folder is looked at for a saved edit where the system
/// has no file events for it (ADR "the server watches files with events").
const WATCH_EVERY: Duration = Duration::from_millis(500);

/// How long requests in flight may take to finish after a stop signal
/// before their connections are closed (RULES.md BL-RUN-10).
pub(crate) const GRACE: Duration = Duration::from_secs(10);

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
    /// The secret git mode's push hook checks, which opens the hook.
    pub(crate) git_hook: Option<Secret<String>>,
}

impl Keys {
    /// The keys `env` holds for the site with `config`.
    pub(crate) fn read(env: &Environment, config: &Config) -> Self {
        let mail = config.mail().map(|mail| &mail.transport);
        Self {
            bucket: env.s3_keys(),
            mail: mail.and_then(|transport| env.mail_secret(transport)),
            git_hook: env.git_hook_secret(),
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
    (output, alert_every): (Output, Every),
) -> Exit {
    let built = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_keep_alive(Duration::from_secs(1))
        .build();
    let (Ok(runtime), Ok(outbound)) = (built, outbound) else {
        return report::failure(output, &serve_message::internal(), Exit::Internal);
    };
    let to = (listen, &outbound, (output, alert_every), keys);
    match serve_site(&runtime, (site, config, routes), start, to, None) {
        Ok(()) if output == Output::Human => report::text("Stopped.\n", Exit::Success),
        Ok(()) => Exit::Success,
        Err((message, exit)) => report::failure(output, &message, exit),
    }
}

/// Serves one site on `runtime` as [`run`] does, or, `among` many, in its
/// place behind the sites folder's listener. Its database opens first and
/// closes last, after every request, the watch, the scheduler and the
/// announcing have stopped.
pub(crate) fn serve_site(
    runtime: &Runtime,
    (site, config, routes): (&Path, &Config, &Routes),
    start: (SiteContent, Assets, FolderLook),
    to: (&Listen, &Outbound, (Output, Every), Keys),
    among: Option<&Among>,
) -> Result<(), (UserMessage, Exit)> {
    let (database, id) = open(site, runtime).map_err(|message| (message, Exit::Failure))?;
    let served = serve_with(
        (site, config, routes),
        start,
        to,
        runtime,
        (&database, id),
        among,
    );
    runtime.block_on(database.close());
    served
}

/// Renders the site with its checked settings and routes, then answers on
/// `listen`, or in its place `among` many, until told to stop.
fn serve_with(
    (site, config, routes): (&Path, &Config, &Routes),
    (checked, assets, look): (SiteContent, Assets, FolderLook),
    (listen, outbound, (output, alert_every), keys): (&Listen, &Outbound, (Output, Every), Keys),
    runtime: &Runtime,
    (database, id): (&Database, SiteId),
    among: Option<&Among>,
) -> Result<(), (UserMessage, Exit)> {
    let (content, hand_back) = (LiveContent::default(), HandBack::default());
    let (redirects, addresses) = redirect_route::stores((database, id));
    let (to, telling, announcing) = destination(
        (site, config, &checked),
        (database, id, outbound),
        (&content, &hand_back),
        (&redirects, &addresses),
    )?;
    let (feed, feeding) = content_feed::start(runtime.handle(), to);
    let (source, first) = first_pages((site, config, routes), feed, &checked)
        .map_err(|message| (message, Exit::Failure))?;
    let security = security_headers()?;
    let search = search_route::Search {
        pages: source.clone(),
        db: database.clone(),
        site: id,
        security: security.clone(),
    };
    let to = (site, config, listen, alert_every);
    let (posted, alerting) = mail_and_alerts(
        to,
        (&search, runtime, &telling),
        &content,
        (keys.mail, outbound),
    )?;
    let mailer = alerting.mailer.clone();
    let served = runtime.block_on(async {
        let front = Door::open((site, listen, among), (output, config)).await?;
        let live = LivePages::new(first);
        hand_back.start(live.replacements());
        let (failed, failure) = oneshot::channel();
        let rest = (source.clone(), live.clone());
        tokio::spawn(render_the_rest(rest, (checked, assets), failed));
        let watching = watch(site, &source, &live, look, WATCH_EVERY);
        let announcing = announcing.retrying(&telling);
        // Each part reads the database or the folder alone, so all open at once.
        let (alerting, alert_watching) = alerting.start((config, database, id, telling.clone()));
        let replies = (security.clone(), telling.clone());
        let commenting = comment_pages::start(config, (database, id), (&source, &live), replies);
        let shown_pages = consent_route::pages(config, live.clone(), security.clone());
        let ((comments, moderation), (door, posts), (media, files), unlocked) = tokio::try_join!(
            async { Ok(commenting.await) },
            publish_door((site, config), (database, id), telling),
            media_library::open(site, config, keys.bucket, outbound),
            unlocking((site, config), (database, id), &source, shown_pages),
        )?;
        let scheduling = scheduler::start(Arc::clone(&door), alerting);
        let previews = preview::router(Arc::clone(&door), source.clone(), config, security.clone());
        let pages = (unlocked, live, config.mount());
        let (desk, pages) =
            redirect_route::start((database, id), pages, (redirects, addresses)).await;
        let (counter, pages) = page_counts::start((database, id), pages, (site, config, outbound));
        let editing = (content.clone(), posts.clone());
        let pages = staff_toolbar::start((database, id, config), editing, pages);
        let (desks, at) = ((moderation, media, desk), (database, id, (site, config)));
        let helpers = (site_hosts::panel(among), mailer);
        let apis = api_router((content, door, posts), desks, at, helpers)
            .merge(push_hook::router(site, keys.git_hook));
        let app = assembled(
            (site, config, &security),
            (apis, files, previews, search_route::router(config, search)),
            (comments, posted, pages),
        );
        let why = Arc::new(Mutex::new(None));
        let stopping = stop_or_fail(failure, Arc::clone(&why));
        let stopped = front.answer(app, stopping).await;
        watching.stop().await;
        tokio::join!(scheduling.stop(), alert_watching.stop());
        tokio::join!(announcing.stop(), counter.stop());
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

/// The routes a reader posts to from a page besides comments, and the
/// alerts ready to start, sharing the one way mail leaves, made from
/// `[mail]` with its `secret` and `outbound`. The routes are the
/// newsletter's and the form block's (G4), whose answers are emailed and
/// told to `telling` (G40), as is each confirmed subscriber; both keep
/// what they receive in the database `site` reads. The alert checks and
/// the speed and uptime watch run `every` so often.
fn mail_and_alerts(
    (folder, config, listen, every): (&Path, &Config, &Listen, Every),
    (site, runtime, telling): (&search_route::Search, &Runtime, &SiteAnnounce),
    content: &LiveContent,
    (secret, outbound): (Option<Secret<String>>, &Outbound),
) -> Result<(Router, alert_watch::Ready), (UserMessage, Exit)> {
    let (database, id) = (&site.db, site.site);
    let internal = |message| (message, Exit::Internal);
    let mailer = newsletter::mailer(config, secret, outbound).map_err(internal)?;
    let newsletters = newsletter::router(
        (folder, config),
        (database, id),
        (runtime, telling),
        mailer.clone(),
    )
    .map_err(internal)?;
    let delivery = SiteDelivery::new(config, mailer.clone(), telling.clone()).map_err(internal)?;
    let delivery: Arc<dyn FormDelivery> = Arc::new(delivery);
    let forms = (&site.security, delivery);
    let forms = form_pages::router(config, (database, id), &site.pages, forms);
    let sources = alert_watch::Sources {
        misses: Arc::new(NotFoundTable::new(database.clone(), id)),
        content: content.clone(),
        https: https_folder(folder, listen),
        settings: config.alerts().clone(),
        visitors: config.visitor_log().on.then(|| {
            let log: Arc<dyn VisitorLogStore> =
                Arc::new(VisitorLogTable::new(database.clone(), id));
            log
        }),
        watch: every.watch.and_then(|looks| {
            let watch = SiteWatch::new(config, outbound, (database, id))?;
            Some((watch, looks))
        }),
    };
    let ready = alert_watch::Ready {
        mailer,
        sources,
        every: every.checks,
    };
    Ok((newsletters.merge(forms), ready))
}

/// The domain and the folder of its certificate, when `listen` serves
/// HTTPS, for the certificate alert.
fn https_folder(site: &Path, listen: &Listen) -> Option<(Domain, HttpsFolder)> {
    match listen {
        Listen::Https(https) => Some((
            https.domain.clone(),
            HttpsFolder::new(serve_https::folder(site)),
        )),
        Listen::Plain(_) => None,
    }
}

/// The site's pages, fed by `feed`, and the first of them to serve: only
/// the home pages are rendered before the server listens, and the rest
/// follows behind the first answer (ADR "khalid serve answers before the
/// full render").
fn first_pages(
    (site, config, routes): (&Path, &Config, &Routes),
    feed: content_feed::ContentFeed,
    checked: &SiteContent,
) -> Result<(SitePages, Pages), UserMessage> {
    let clock = Arc::new(SystemClock);
    let source = SitePages::new(site, config.clone(), routes.clone(), Some(feed), clock)?;
    let first = source.render_first(checked)?;
    let first = Pages::first(first).map_err(|_| serve_message::internal())?;
    let first = with_hint(first, &source);
    Ok((source, first))
}

/// The Content API and the Admin API as `khalid serve` runs them, with the
/// comment moderation, media library, stored redirects, webhooks and staff
/// accounts of site `id`, the post checks, schema templates, pages,
/// snippets, authors, tags, categories and settings of the site in `site`,
/// the super-admin routes over `panel`, and comments on drafts emailed
/// through `mailer` when the site sends mail.
fn api_router(
    (content, door, posts): (LiveContent, Arc<PublishDoor>, SitePosts),
    (comments, media, redirects): (Arc<dyn CommentModeration>, MediaLibrary, RedirectDesk),
    (database, id, (site, config)): (&Database, SiteId, (&Path, &Config)),
    (panel, mailer): (Arc<dyn SiteDirectory>, Option<Arc<dyn Mailer>>),
) -> Router {
    let accounts = staff::accounts(database, id, config);
    let sessions: Arc<dyn SessionCheck> = Arc::new(accounts.clone());
    let routes = khalid_auth::staff_routes(accounts.clone());
    let staff = (khalid_auth::with_sites(routes, accounts, panel), sessions);
    let desks = AdminDesks {
        comments,
        media: Some(media),
        redirects,
        webhooks: Some(Arc::new(WebhookTable::new(
            database.clone(),
            id,
            SystemClock,
        ))),
        trash: Some(Arc::new(posts)),
        audit: Some(AuditDesk::new(
            Arc::new(AuditTable::new(database.clone(), id)),
            SystemClock,
        )),
        checks: Some(Arc::new(SitePosts::new(site, config.clone(), SystemClock))),
        schema_templates: Some(Arc::new(SiteSchemaTemplates::new(site, config))),
        pages: Some(Arc::new(SiteEntries::<AdminPage>::new(site, config))),
        snippets: Some(Arc::new(SiteEntries::<AdminSnippet>::new(site, config))),
        authors: Some(Arc::new(SiteEntries::<AdminAuthor>::new(site, config))),
        tags: Some(Arc::new(SiteTags::new(site, config))),
        categories: Some(Arc::new(SiteCategories::new(site, config))),
        settings: Some(Arc::new(SiteSettings::new(site, config))),
        analytics: AnalyticsDesk::new(
            Arc::new(AnalyticsTable::new(database.clone(), id)),
            Arc::new(SystemClock),
        ),
        draft_comments: Some(draft_comment_mail::desk((database, id, config), mailer)),
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

/// Where the Content API's feed writes, as `khalid serve` sets it up, and
/// the site's announcements, sent through `outbound`. The feed tells them
/// of each entry saved since the run began with the `checked` content.
///
/// # Errors
///
/// Returns a message when the `IndexNow` key file's address cannot be made
/// from the site's address.
fn destination(
    (site, config, checked): (&Path, &Config, &SiteContent),
    (database, id, outbound): (&Database, SiteId, &Outbound),
    (live, hand_back): (&LiveContent, &HandBack),
    (redirects, addresses): (
        &crate::live_redirects::LiveRedirects,
        &crate::site_addresses::LiveAddresses,
    ),
) -> Result<(Destination, SiteAnnounce, Announcing), (UserMessage, Exit)> {
    let (telling, announcing) = announce::once(config, (database, id), outbound)
        .map_err(|message| (message, Exit::Failure))?;
    let to = Destination {
        folder: site.to_path_buf(),
        config: config.clone(),
        db: database.clone(),
        site: id,
        live: live.clone(),
        hand_back: hand_back.clone(),
        addresses: addresses.clone(),
        redirects: redirects.clone(),
        announce: Some(telling.clone()),
        started: (
            Entries::of(checked),
            Tags::of(config, checked),
            Settings::read(site),
        ),
        pings: content_feed::UpdatePings::default(),
    };
    to.pings.set(telling.clone());
    Ok((to, telling, announcing))
}

/// The one router the server answers: the APIs, the site's own routes, the
/// drawn share cards, the admin, the comments, the newsletters and the
/// forms.
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
        .merge(consent_route::router(config, security.clone()))
        .merge(pages)
}

/// The publish door the Admin API, the scheduler and preview links share.
/// It keeps schedules in the database and tells `telling` about each post
/// that goes live.
async fn publish_door(
    (site, config): (&Path, &Config),
    (database, id): (&Database, SiteId),
    telling: SiteAnnounce,
) -> Result<(Arc<PublishDoor>, SitePosts), (UserMessage, Exit)> {
    let file = db::data_file(site).map_err(|message| (message, Exit::Failure))?;
    let posts = SitePosts::new(site, config.clone(), SystemClock);
    let schedules = Arc::new(ScheduleTable::new(database.clone(), id));
    let signer = preview::signer(config, (database, id, &file)).await;
    let door = PublishDoor::new(Arc::new(posts.clone()), Arc::new(telling))
        .with_schedules(schedules, SystemClock)
        .with_previews(signer.map_err(|message| (message, Exit::Failure))?);
    Ok((Arc::new(door), posts))
}

/// `pages`, the site's page router, with the password form and the cookie
/// checks of `source`'s password posts and posts with gated content. The
/// cookies are signed with the site's preview key, each under its own
/// label, and marked `Secure` when the
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
    let reader = newsletter::reader_pass(config, &key);
    Ok(unlock::router(
        config,
        source.gate(),
        (signer, reader),
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
/// providers and the reader script by its hash.
fn security_headers() -> Result<SecurityHeaders, (UserMessage, Exit)> {
    let origins: Vec<&str> = EMBED_PROVIDERS.iter().map(|p| p.frame_origin()).collect();
    SecurityHeaders::new(&origins)
        .and_then(|headers| headers.with_script(&assets::reader().integrity))
        .map_err(|_| (serve_message::internal(), Exit::Internal))
}

/// Says where the site answers. In JSON, this one line is the whole output.
pub(crate) fn announce(output: Output, config: &Config, address: &str) {
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
pub(crate) async fn stop() {
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
