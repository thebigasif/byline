//! Webhooks: the events Khalid announces to other servers, how a delivery
//! is signed and logged, and how an incoming webhook is checked.
//!
//! This crate owns the event catalogue ([`Event`]), the `Khalid-Signature`
//! header ([`sign`], [`SigningKey`]), the [`Verifier`] for incoming webhooks
//! (RULES.md BL-SEC-7), and the delivery log: each [`Delivery`] with its
//! attempts and the [`RetrySchedule`] over 24 hours. It must not know about
//! content files, templates or SQL.

mod delivery;
mod event;
mod retry_schedule;
mod signature;
mod verifier;

pub use delivery::{Attempt, Delivery, DeliveryId, DeliveryState, NewDelivery, Outcome, WebhookId};
pub use event::{Event, ParseEventError};
pub use retry_schedule::RetrySchedule;
pub use signature::{SIGNATURE_HEADER, Signature, SigningKey, SigningKeyError, sign};
pub use verifier::{Verifier, VerifyError};
