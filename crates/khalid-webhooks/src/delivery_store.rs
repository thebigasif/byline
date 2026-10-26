//! Where the delivery log is kept.

use std::error::Error;
use std::future::Future;

use chrono::{DateTime, Utc};

use crate::{Delivery, NewDelivery, WebhookId};

/// The delivery log's storage. `khalid-db` implements it on SQLite in task
/// 1.15. [`MemoryDeliveryStore`], from the `test-util` feature, keeps it in
/// memory for tests.
pub trait DeliveryStore: Send + Sync {
    /// Stores a new delivery and returns it with its identifier.
    ///
    /// # Errors
    ///
    /// Returns [`DeliveryStoreError`] when the log cannot be written.
    fn insert(
        &self,
        new: NewDelivery,
    ) -> impl Future<Output = Result<Delivery, DeliveryStoreError>> + Send;

    /// Replaces a stored delivery with `delivery`, after an attempt.
    ///
    /// # Errors
    ///
    /// Returns [`DeliveryStoreError`] when the log cannot be written or has
    /// no delivery with that identifier.
    fn update(
        &self,
        delivery: &Delivery,
    ) -> impl Future<Output = Result<(), DeliveryStoreError>> + Send;

    /// Every pending delivery whose next attempt is due at `now`, oldest
    /// first.
    ///
    /// # Errors
    ///
    /// Returns [`DeliveryStoreError`] when the log cannot be read.
    fn due(
        &self,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<Vec<Delivery>, DeliveryStoreError>> + Send;

    /// Every delivery for `webhook`, newest first, for the admin's log.
    ///
    /// # Errors
    ///
    /// Returns [`DeliveryStoreError`] when the log cannot be read.
    fn log(
        &self,
        webhook: WebhookId,
    ) -> impl Future<Output = Result<Vec<Delivery>, DeliveryStoreError>> + Send;
}

/// The delivery log could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DeliveryStoreError {
    /// Reading failed.
    #[error("the delivery log could not be read")]
    Read {
        /// What the storage reported.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
    /// Writing failed.
    #[error("the delivery log could not be written")]
    Write {
        /// What the storage reported.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryDeliveryStore;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::collections::BTreeMap;
    use std::future::{self, Future};
    use std::sync::{Mutex, MutexGuard};

    use chrono::{DateTime, Utc};

    use super::{DeliveryStore, DeliveryStoreError};
    use crate::{Delivery, DeliveryId, NewDelivery, WebhookId};

    /// The delivery log in memory, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryDeliveryStore {
        rows: Mutex<BTreeMap<DeliveryId, Delivery>>,
    }

    /// A lock was poisoned by a panic in another test thread.
    #[derive(Debug, thiserror::Error)]
    #[error("the in-memory delivery log was poisoned by a panic")]
    struct Poisoned;

    /// Stands in for a database row that is not there.
    #[derive(Debug, thiserror::Error)]
    #[error("there is no delivery {0}")]
    struct Missing(i64);

    impl MemoryDeliveryStore {
        fn rows(&self) -> Result<MutexGuard<'_, BTreeMap<DeliveryId, Delivery>>, Poisoned> {
            self.rows.lock().map_err(|_| Poisoned)
        }

        fn insert_now(&self, new: NewDelivery) -> Result<Delivery, DeliveryStoreError> {
            let mut rows = self.rows().map_err(write)?;
            let next = rows.last_key_value().map_or(1, |(id, _)| id.get() + 1);
            let delivery = new.into_delivery(DeliveryId::new(next));
            rows.insert(delivery.id(), delivery.clone());
            Ok(delivery)
        }

        fn update_now(&self, delivery: &Delivery) -> Result<(), DeliveryStoreError> {
            let mut rows = self.rows().map_err(write)?;
            let row = rows
                .get_mut(&delivery.id())
                .ok_or_else(|| write(Missing(delivery.id().get())))?;
            *row = delivery.clone();
            Ok(())
        }

        fn select(
            &self,
            keep: impl Fn(&Delivery) -> bool,
            order: Order,
        ) -> Result<Vec<Delivery>, DeliveryStoreError> {
            let rows = self.rows().map_err(read)?;
            let kept = rows.values().filter(|row| keep(row)).cloned();
            Ok(match order {
                Order::OldestFirst => kept.collect(),
                Order::NewestFirst => kept.rev().collect(),
            })
        }
    }

    /// The order rows come back in.
    #[derive(Debug, Clone, Copy)]
    enum Order {
        OldestFirst,
        NewestFirst,
    }

    fn read(error: Poisoned) -> DeliveryStoreError {
        DeliveryStoreError::Read {
            source: Box::new(error),
        }
    }

    fn write(error: impl std::error::Error + Send + Sync + 'static) -> DeliveryStoreError {
        DeliveryStoreError::Write {
            source: Box::new(error),
        }
    }

    /// Memory answers at once, so each future is ready when returned.
    impl DeliveryStore for MemoryDeliveryStore {
        fn insert(
            &self,
            new: NewDelivery,
        ) -> impl Future<Output = Result<Delivery, DeliveryStoreError>> + Send {
            future::ready(self.insert_now(new))
        }

        fn update(
            &self,
            delivery: &Delivery,
        ) -> impl Future<Output = Result<(), DeliveryStoreError>> + Send {
            future::ready(self.update_now(delivery))
        }

        fn due(
            &self,
            now: DateTime<Utc>,
        ) -> impl Future<Output = Result<Vec<Delivery>, DeliveryStoreError>> + Send {
            future::ready(self.select(|row| row.is_due(now), Order::OldestFirst))
        }

        fn log(
            &self,
            webhook: WebhookId,
        ) -> impl Future<Output = Result<Vec<Delivery>, DeliveryStoreError>> + Send {
            let mine = |row: &Delivery| row.webhook() == webhook;
            future::ready(self.select(mine, Order::NewestFirst))
        }
    }
}
