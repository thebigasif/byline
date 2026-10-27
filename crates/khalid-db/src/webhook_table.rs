//! The site's webhook list on SQLite, for `khalid serve` to announce
//! events to and for whatever adds webhooks.

use std::sync::Arc;

use chrono::SecondsFormat;
use khalid_core::{AbsoluteUrl, Clock, Secret};
use khalid_webhooks::{Event, SigningKey, Webhook, WebhookId};
use sqlx::Row;

use crate::{Database, DbError, SiteId};

/// The length of a webhook's secret in bytes: 256 random bits, as RULES.md
/// BL-SEC-3 asks of tokens. The `webhooks` table checks it too.
const SECRET_BYTES: usize = 32;

/// One site's webhooks in `webhooks`. Each row sends one event to one
/// address. The secret is kept, not hashed, because every delivery is
/// signed with it, and it is never logged. Cloning shares the connections.
#[derive(Debug, Clone)]
pub struct WebhookTable {
    db: Database,
    site: SiteId,
    clock: Arc<dyn Clock>,
}

impl WebhookTable {
    /// The webhooks of `site` in `db`, dating new ones with `clock`.
    pub fn new(db: Database, site: SiteId, clock: impl Clock + 'static) -> Self {
        Self {
            db,
            site,
            clock: Arc::new(clock),
        }
    }

    /// The webhooks that are switched on, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Query`] when the table cannot be read, and
    /// [`DbError::Unreadable`] when a row holds an event, address or secret
    /// this version cannot read.
    pub async fn list(&self) -> Result<Vec<Webhook>, DbError> {
        let rows = sqlx::query(
            "SELECT id, event, url, secret FROM webhooks \
             WHERE site_id = ?1 AND disabled_at IS NULL ORDER BY id",
        )
        .bind(self.site.as_row())
        .fetch_all(&self.db.readers)
        .await?;
        rows.iter()
            .map(|row| {
                let (id, event, url, secret): (i64, String, String, Vec<u8>) = (
                    row.try_get("id")?,
                    row.try_get("event")?,
                    row.try_get("url")?,
                    row.try_get("secret")?,
                );
                let event = event
                    .parse::<Event>()
                    .map_err(|_| unreadable("event", &event))?;
                let url = AbsoluteUrl::parse(&url).map_err(|_| unreadable("url", &url))?;
                let key = SigningKey::try_new(&Secret::new(to_hex(&secret)))
                    .map_err(|_| unreadable("secret", "a secret of the wrong length"))?;
                Ok(Webhook::new(WebhookId::new(id), url, key, vec![event]))
            })
            .collect()
    }

    /// Adds a webhook that sends `event` to `url`, signed with a new random
    /// secret. Returns the webhook and the secret, which the receiver needs
    /// to check signatures; show it once, and never log it.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::NoRandomness`] when the system has no randomness
    /// to make a secret, and [`DbError::Query`] when the row cannot be
    /// written.
    pub async fn create(
        &self,
        url: &AbsoluteUrl,
        event: Event,
    ) -> Result<(Webhook, Secret<String>), DbError> {
        let mut bytes = [0_u8; SECRET_BYTES];
        getrandom::fill(&mut bytes).map_err(|source| DbError::NoRandomness { source })?;
        let created_at = self.clock.now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO webhooks (site_id, event, url, secret, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
        )
        .bind(self.site.as_row())
        .bind(event.as_str())
        .bind(url.as_str())
        .bind(bytes.as_slice())
        .bind(created_at)
        .fetch_one(&self.db.writer)
        .await?;
        let secret = Secret::new(to_hex(&bytes));
        let key = SigningKey::try_new(&secret)
            .map_err(|_| unreadable("secret", "a secret of the wrong length"))?;
        let webhook = Webhook::new(WebhookId::new(id), url.clone(), key, vec![event]);
        Ok((webhook, secret))
    }
}

/// `bytes` as lowercase hex digits: the secret as the receiver sees it.
fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

fn unreadable(column: &'static str, value: &str) -> DbError {
    DbError::Unreadable {
        column,
        value: value.to_owned(),
    }
}
