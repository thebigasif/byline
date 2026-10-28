//! `khalid webhooks add`, `list` and `remove`: the site's webhooks from the
//! command line, through the same `WebhookStore` the Admin API uses
//! (KH-API-3). A secret is shown once, when its webhook is added.

use std::fmt::Write;
use std::path::Path;

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_api::webhook_store::{StoredWebhook, WebhookStore, WebhookStoreError};
use khalid_core::{AbsoluteUrl, Secret, Slug, SystemClock, UserMessage};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::Event;
use serde::Serialize;

use crate::cli::{Output, WebhooksCommand};
use crate::exit::Exit;
use crate::{db, db_message, report};

/// One webhook as `--json` prints it. `secret` appears only when the
/// webhook is added.
#[derive(Serialize)]
struct WebhookBody<'a> {
    id: i64,
    event: &'static str,
    url: &'a str,
    created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<&'a str>,
}

impl<'a> From<&'a StoredWebhook> for WebhookBody<'a> {
    fn from(stored: &'a StoredWebhook) -> Self {
        Self {
            id: stored.id,
            event: stored.event.as_str(),
            url: stored.url.as_str(),
            created_at: stored.created_at,
            secret: None,
        }
    }
}

/// The JSON shape of `khalid webhooks list --json`.
#[derive(Serialize)]
struct ListedBody<'a> {
    webhooks: Vec<WebhookBody<'a>>,
}

/// The JSON shape of `khalid webhooks remove --json`.
#[derive(Serialize)]
struct RemovedBody {
    id: i64,
    removed: bool,
}

/// Runs a webhook command on the `site` folder. Only its database is
/// read, not its content, so a large site answers at once.
pub(crate) fn run(site: &Path, command: &WebhooksCommand, output: Output) -> Exit {
    let done = match command {
        WebhooksCommand::Add { event, url } => match AbsoluteUrl::parse(url) {
            Ok(url) => on_store(site, async |store| add(store, &url, *event, output).await),
            Err(_) => Err((bad_url(), Exit::Usage)),
        },
        WebhooksCommand::List => on_store(site, async |store| list(store, output).await),
        WebhooksCommand::Remove { id } => {
            on_store(site, async |store| remove(store, *id, output).await)
        }
    };
    done.flatten()
        .unwrap_or_else(|(message, exit)| report::failure(output, &message, exit))
}

type Done = Result<Exit, (UserMessage, Exit)>;

/// Adds the webhook and shows its secret: the one time it is ever shown.
async fn add(store: &dyn WebhookStore, url: &AbsoluteUrl, event: Event, output: Output) -> Done {
    let (added, secret) = store.add(url, event).await.map_err(unreachable)?;
    Ok(match output {
        Output::Json => {
            let body = WebhookBody {
                secret: Some(secret.expose()),
                ..WebhookBody::from(&added)
            };
            report::json(&body, Exit::Success)
        }
        Output::Human => report::text(&added_text(&added, &secret), Exit::Success),
    })
}

/// What a person reads once a webhook is added.
fn added_text(added: &StoredWebhook, secret: &Secret<String>) -> String {
    format!(
        "Added webhook {id}: it sends {event} to {url}.\n\
         Each delivery is signed with this secret. This is the only time Khalid shows it, \
         so copy it now into the receiver:\n\n    {secret}\n\n\
         To stop it, run `khalid webhooks remove {id}`.\n",
        id = added.id,
        event = added.event,
        url = added.url.as_str(),
        secret = secret.expose(),
    )
}

/// Lists every webhook that is on, oldest first.
async fn list(store: &dyn WebhookStore, output: Output) -> Done {
    let stored = store.list().await.map_err(unreachable)?;
    Ok(match output {
        Output::Json => {
            let webhooks = stored.iter().map(WebhookBody::from).collect();
            report::json(&ListedBody { webhooks }, Exit::Success)
        }
        Output::Human if stored.is_empty() => report::text(
            "This site has no webhooks. Add one with `khalid webhooks add --event <event> --url <address>`.\n",
            Exit::Success,
        ),
        Output::Human => {
            let mut lines = String::new();
            for w in &stored {
                let since = w.created_at.to_rfc3339_opts(SecondsFormat::Secs, true);
                let (id, event, url) = (w.id, w.event, w.url.as_str());
                let _ = writeln!(lines, "{id}  {event}  {url}  added {since}");
            }
            report::text(&lines, Exit::Success)
        }
    })
}

/// Switches off webhook `id`.
async fn remove(store: &dyn WebhookStore, id: i64, output: Output) -> Done {
    if !store.remove(id).await.map_err(unreachable)? {
        return Err((unknown(id), Exit::Failure));
    }
    Ok(match output {
        Output::Json => report::json(&RemovedBody { id, removed: true }, Exit::Success),
        Output::Human => report::text(
            &format!("Switched off webhook {id}. Nothing more is sent to it.\n"),
            Exit::Success,
        ),
    })
}

/// Opens the site's database, brings it up to date, runs `work` on the
/// webhooks of the site, then closes the database.
fn on_store<T>(
    site: &Path,
    work: impl AsyncFnOnce(&dyn WebhookStore) -> T,
) -> Result<T, (UserMessage, Exit)> {
    let failed = |message| (message, Exit::Failure);
    let path = db::data_file(site).map_err(failed)?;
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(db::MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err(failed(db_message::internal()));
    };
    runtime.block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|error| failed(db_message::of(&path, &error)))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        }
        .await;
        let done = match opened {
            Ok(id) => Ok(work(&WebhookTable::new(database.clone(), id, SystemClock)).await),
            Err(error) => Err(failed(db_message::of(&path, &error))),
        };
        database.close().await;
        done
    })
}

/// The address is not repeated, since it may carry a password.
fn bad_url() -> UserMessage {
    UserMessage::new(
        "The --url is not an address Khalid can send to.",
        "Deliveries go over the web, so the address needs http or https, a host, \
         and no user name or password.",
        "Give the full address, such as --url https://hooks.example.com/khalid.",
    )
}

fn unknown(id: i64) -> UserMessage {
    UserMessage::new(
        format!("No webhook that is on has the identity {id}."),
        "It may be switched off already, or the number may be mistyped.",
        "Run `khalid webhooks list` to see each webhook's identity.",
    )
}

/// The answer when the webhooks in `data/site.db` cannot be read or changed.
fn unreachable(_: WebhookStoreError) -> (UserMessage, Exit) {
    let message = UserMessage::new(
        "Khalid could not read or change the site's webhooks in data/site.db.",
        "Nothing was changed, so every webhook works as it did before.",
        "Check the disk has free space and nothing else holds data/site.db, then run it again.",
    );
    (message, Exit::Failure)
}
