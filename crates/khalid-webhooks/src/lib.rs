//! Webhooks: the events Khalid announces to other servers, how a delivery
//! is signed and logged, and how an incoming webhook is checked.
//!
//! This crate owns the event catalogue ([`Event`]), the `Khalid-Signature`
//! header ([`sign`], [`SigningKey`]), the [`Verifier`] for incoming webhooks
//! (RULES.md BL-SEC-7), and the delivery log: each [`Delivery`] with its
//! attempts, the [`RetrySchedule`] over 24 hours, and the [`DeliveryStore`]
//! that keeps them. It must not know about content files, templates or SQL;
//! `khalid-db` implements the store.

mod delivery;
mod delivery_store;
mod event;
mod retry_schedule;
mod signature;
mod verifier;

pub use delivery::{Attempt, Delivery, DeliveryId, DeliveryState, NewDelivery, Outcome, WebhookId};
#[cfg(any(test, feature = "test-util"))]
pub use delivery_store::MemoryDeliveryStore;
pub use delivery_store::{DeliveryStore, DeliveryStoreError};
pub use event::{Event, ParseEventError};
pub use retry_schedule::RetrySchedule;
pub use signature::{SIGNATURE_HEADER, Signature, SigningKey, SigningKeyError, sign};
pub use verifier::{Verifier, VerifyError};
