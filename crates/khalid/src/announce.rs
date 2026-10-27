//! What `khalid serve` tells the world when a post goes live, is
//! scheduled or comes down through the publish door: the matching `post.*`
//! webhook to every webhook in the site's list that asked for it, and the
//! post's address to `IndexNow` when `[seo.indexnow]` switches it on (CTO
//! answers 2026-10-27, section 4).
//! Failed deliveries are tried again on a timer, to the webhook list as it
//! stands at each round.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use khalid_api::publish_door::Announce;
use khalid_api::resources::AdminPost;
use khalid_core::{AbsoluteUrl, Config, SystemClock, UserMessage};
use khalid_db::{Database, DeliveryLog, SiteId, WebhookTable};
use khalid_outbound::Outbound;
use khalid_seo::{IndexNow, Retries};
use khalid_server::{Background, retry_deliveries};
use khalid_webhooks::{Dispatcher, Event};
use tokio::task::JoinSet;

use crate::{build_message, build_pages, build_seo, db_message};

/// How often failed webhook deliveries are looked at for one that is due.
const RETRY_EVERY: Duration = Duration::from_mins(1);

/// How long announcements still being sent may take to finish once the
/// server stops (RULES.md BL-RUN-10).
const GRACE: Duration = Duration::from_secs(10);

/// Who to tell, shared by every announcement.
struct Receivers {
    config: Config,
    webhooks: WebhookTable,
    dispatcher: Arc<Dispatcher<DeliveryLog>>,
    indexnow: Option<IndexNow>,
    outbound: Outbound,
}

/// The announcements in flight.
type Sending = Arc<Mutex<JoinSet<()>>>;

/// The publish door's [`Announce`] for a running site. It sends each post
/// on a task of its own and returns at once.
#[derive(Clone)]
pub(crate) struct SiteAnnounce {
    receivers: Arc<Receivers>,
    sending: Sending,
}

impl SiteAnnounce {
    /// Sends `event` about `post` on a task of its own.
    fn tell(&self, event: Event, post: &AdminPost) {
        let mut sending = self.sending.lock().unwrap_or_else(PoisonError::into_inner);
        while sending.try_join_next().is_some() {}
        sending.spawn(announce(Arc::clone(&self.receivers), event, post.clone()));
    }
}

impl Announce for SiteAnnounce {
    fn published(&self, post: &AdminPost) {
        self.tell(Event::PostPublished, post);
    }

    fn scheduled(&self, post: &AdminPost) {
        self.tell(Event::PostScheduled, post);
    }

    fn unpublished(&self, post: &AdminPost) {
        self.tell(Event::PostUnpublished, post);
    }
}

/// The announcing jobs, stopped before the database closes.
#[derive(Debug)]
pub(crate) struct Announcing {
    sending: Sending,
    retrying: Background,
}

impl Announcing {
    /// Stops retrying, and gives announcements in flight [`GRACE`] to
    /// finish. A webhook delivery cut off is in the log, and is tried
    /// again after the next start.
    pub(crate) async fn stop(self) {
        self.retrying.stop().await;
        let sending =
            std::mem::take(&mut *self.sending.lock().unwrap_or_else(PoisonError::into_inner));
        if tokio::time::timeout(GRACE, sending.join_all())
            .await
            .is_err()
        {
            tracing::warn!("announcements still sending were cut off as the server stopped");
        }
    }
}

/// Starts announcing for the site with `config` in `db`, whose file is
/// `file`, sending through `outbound`. Must be called inside the tokio
/// runtime.
///
/// # Errors
///
/// Returns a message when the webhook list cannot be read, or the
/// `IndexNow` key file's address cannot be made from the site's address.
pub(crate) async fn start(
    config: &Config,
    (db, site, file): (&Database, SiteId, &Path),
    outbound: &Outbound,
) -> Result<(SiteAnnounce, Announcing), UserMessage> {
    let webhooks = WebhookTable::new(db.clone(), site, SystemClock);
    webhooks
        .list()
        .await
        .map_err(|error| db_message::of(file, &error))?;
    let log = DeliveryLog::new(db.clone(), site);
    let site_address = config.address().clone();
    let dispatcher = Arc::new(Dispatcher::new(
        outbound.clone(),
        log,
        SystemClock,
        site_address,
    ));
    let indexnow = match &config.seo().indexnow {
        Some(settings) => Some(
            IndexNow::new(&build_seo::site(config)?, settings)
                .map_err(|error| build_message::seo(&error))?,
        ),
        None => None,
    };
    let list = {
        let webhooks = webhooks.clone();
        move || {
            let webhooks = webhooks.clone();
            async move { webhooks.list().await }
        }
    };
    let retrying = retry_deliveries(Arc::clone(&dispatcher), list, RETRY_EVERY);
    let receivers = Arc::new(Receivers {
        config: config.clone(),
        webhooks,
        dispatcher,
        indexnow,
        outbound: outbound.clone(),
    });
    let sending = Sending::default();
    let telling = SiteAnnounce {
        receivers,
        sending: Arc::clone(&sending),
    };
    Ok((telling, Announcing { sending, retrying }))
}

/// Fires `event` about `post` and, when the post went live or came down,
/// sends its address to `IndexNow`. A failure is logged; a failed webhook
/// delivery is retried from the log.
async fn announce(receivers: Arc<Receivers>, event: Event, post: AdminPost) {
    // The body of a post event: the post in Admin API shape.
    let body = serde_json::json!({ "post": &post });
    let webhooks = async {
        let listed = receivers.webhooks.list().await.map_err(|e| e.to_string())?;
        let sent = receivers.dispatcher.publish(&listed, event, &body).await;
        sent.map_err(|e| e.to_string())
    };
    let search = async {
        let Some(indexnow) = &receivers.indexnow else {
            return Ok(());
        };
        if event == Event::PostScheduled {
            return Ok(());
        }
        let url = post_url(&receivers.config, &post).ok_or("the post has no address")?;
        let sent = indexnow
            .send(&receivers.outbound, &Retries::default(), &[url])
            .await;
        sent.map(|_| ())
            .map_err(|_| "IndexNow did not take the address")
    };
    let (webhooks, search) = tokio::join!(webhooks, search);
    if let Err(error) = webhooks {
        tracing::warn!(id = post.id(), %event, %error, "a post event could not be logged or sent");
    }
    if let Err(error) = search {
        tracing::warn!(id = post.id(), %error, "the post's address did not reach IndexNow");
    }
}

/// The public address of the post with identity `<language>.<slug>`.
fn post_url(config: &Config, post: &AdminPost) -> Option<AbsoluteUrl> {
    let (language, _) = post.id().split_once('.')?;
    let locale = config.languages().iter().find(|l| l.as_str() == language)?;
    let path = build_pages::path_of(config, locale, &format!("{}/", post.slug()));
    let base = config.address().as_str().trim_end_matches('/');
    AbsoluteUrl::parse(&format!("{base}{path}")).ok()
}
