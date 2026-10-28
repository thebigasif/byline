//! Where the site's webhooks are kept, for the Admin API's webhook routes
//! and `khalid webhooks` (KH-API-3).

use std::error::Error;

use chrono::{DateTime, Utc};
use khalid_core::{AbsoluteUrl, Secret};
use khalid_webhooks::Event;

use crate::content_store::BoxFuture;

/// A webhook as listed: what it sends and where. Its secret is never
/// listed; it is shown once, when the webhook is added.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StoredWebhook {
    /// The identity the store gave it.
    pub id: i64,
    /// The event it sends.
    pub event: Event,
    /// Where it sends it.
    pub url: AbsoluteUrl,
    /// When it was added.
    pub created_at: DateTime<Utc>,
}

impl StoredWebhook {
    /// A webhook as a store reads it back.
    pub fn new(id: i64, event: Event, url: AbsoluteUrl, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            event,
            url,
            created_at,
        }
    }
}

/// The site's webhooks. `khalid-db` keeps them in SQLite as visitor data;
/// `MemoryWebhookStore`, from the `test-util` feature, keeps them in memory
/// for tests.
pub trait WebhookStore: Send + Sync {
    /// Every webhook that is switched on, oldest first.
    ///
    /// # Errors
    ///
    /// [`WebhookStoreError`] when they cannot be read.
    fn list(&self) -> BoxFuture<'_, Result<Vec<StoredWebhook>, WebhookStoreError>>;

    /// Adds a webhook that sends `event` to `url`, with a new random
    /// secret. Returns it and the secret, which is never shown again.
    ///
    /// # Errors
    ///
    /// [`WebhookStoreError`] when it cannot be written.
    fn add<'a>(
        &'a self,
        url: &'a AbsoluteUrl,
        event: Event,
    ) -> BoxFuture<'a, Result<(StoredWebhook, Secret<String>), WebhookStoreError>>;

    /// Switches off webhook `id`, answering whether it was on. Its
    /// deliveries stay in the log.
    ///
    /// # Errors
    ///
    /// [`WebhookStoreError`] when it cannot be written.
    fn remove(&self, id: i64) -> BoxFuture<'_, Result<bool, WebhookStoreError>>;
}

/// The webhooks could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the webhooks could not be read or written")]
pub struct WebhookStoreError(#[source] Box<dyn Error + Send + Sync>);

impl WebhookStoreError {
    /// Wraps what the storage reported.
    pub fn storage(source: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryWebhookStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::sync::{Mutex, PoisonError};

    use chrono::{DateTime, Utc};
    use khalid_core::{AbsoluteUrl, Secret};
    use khalid_webhooks::Event;

    use super::{BoxFuture, StoredWebhook, WebhookStore, WebhookStoreError};

    /// Webhooks held in memory, for tests, dated `at` and given secrets
    /// made of their identity, which protect nothing.
    #[derive(Debug)]
    pub struct MemoryWebhookStore {
        at: DateTime<Utc>,
        webhooks: Mutex<Vec<StoredWebhook>>,
    }

    impl MemoryWebhookStore {
        /// No webhooks yet; each one added is dated `at`.
        pub fn new(at: DateTime<Utc>) -> Self {
            Self {
                at,
                webhooks: Mutex::default(),
            }
        }

        fn webhooks(&self) -> std::sync::MutexGuard<'_, Vec<StoredWebhook>> {
            self.webhooks.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl WebhookStore for MemoryWebhookStore {
        fn list(&self) -> BoxFuture<'_, Result<Vec<StoredWebhook>, WebhookStoreError>> {
            Box::pin(std::future::ready(Ok(self.webhooks().clone())))
        }

        fn add<'a>(
            &'a self,
            url: &'a AbsoluteUrl,
            event: Event,
        ) -> BoxFuture<'a, Result<(StoredWebhook, Secret<String>), WebhookStoreError>> {
            let mut webhooks = self.webhooks();
            let id = webhooks.iter().map(|w| w.id).max().unwrap_or(0) + 1;
            let webhook = StoredWebhook::new(id, event, url.clone(), self.at);
            webhooks.push(webhook.clone());
            let secret = Secret::new(format!("{id:064x}"));
            Box::pin(std::future::ready(Ok((webhook, secret))))
        }

        fn remove(&self, id: i64) -> BoxFuture<'_, Result<bool, WebhookStoreError>> {
            let mut webhooks = self.webhooks();
            let before = webhooks.len();
            webhooks.retain(|w| w.id != id);
            let removed = webhooks.len() < before;
            Box::pin(std::future::ready(Ok(removed)))
        }
    }
}
