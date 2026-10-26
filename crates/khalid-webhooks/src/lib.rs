//! Webhooks: the events Khalid announces to other servers, how a delivery
//! is signed, and how an incoming webhook is checked.
//!
//! This crate owns the event catalogue ([`Event`]), the `Khalid-Signature`
//! header ([`sign`], [`SigningKey`]) and the [`Verifier`] for incoming
//! webhooks (RULES.md BL-SEC-7). Delivery joins it in the next part of task
//! 1.19. It must not know about content files, templates or SQL.

mod event;
mod signature;
mod verifier;

pub use event::{Event, ParseEventError};
pub use signature::{SIGNATURE_HEADER, Signature, SigningKey, SigningKeyError, sign};
pub use verifier::{Verifier, VerifyError};
