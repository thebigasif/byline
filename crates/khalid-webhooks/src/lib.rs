//! Webhooks: the events Khalid announces to other servers, how a delivery
//! is signed and logged, and how an incoming webhook is checked.
//!
//! This crate owns the event catalogue ([`Event`]), the `Khalid-Signature`
//! header ([`sign`], [`SigningKey`]), the [`Verifier`] for incoming webhooks
//! (RULES.md BL-SEC-7), the delivery log: each [`Delivery`] with its
//! attempts, the [`RetrySchedule`] over 24 hours and the [`DeliveryStore`]
//! that keeps them, and the [`Dispatcher`] that sends each [`Webhook`] its
//! events through the outbound client (BL-RUN-8). It must not know about
//! content files, templates or SQL; `khalid-db` implements the store.

mod delivery;
mod delivery_store;
mod dispatcher;
mod event;
mod retry_schedule;
mod signature;
mod verifier;
mod webhook;

pub use delivery::{Attempt, Delivery, DeliveryId, DeliveryState, NewDelivery, Outcome, WebhookId};
#[cfg(any(test, feature = "test-util"))]
pub use delivery_store::MemoryDeliveryStore;
pub use delivery_store::{DeliveryStore, DeliveryStoreError};
pub use dispatcher::{DispatchError, Dispatcher};
pub use event::{Event, ParseEventError};
pub use retry_schedule::RetrySchedule;
pub use signature::{SIGNATURE_HEADER, Signature, SigningKey, SigningKeyError, sign};
pub use verifier::{Verifier, VerifyError};
pub use webhook::Webhook;
