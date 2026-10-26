//! Webhooks: the events Khalid announces to other servers, and how a
//! delivery is signed.
//!
//! This crate owns the event catalogue ([`Event`]) and the
//! `Khalid-Signature` header ([`sign`], [`SigningKey`]). Delivery and the
//! verifier for incoming webhooks join it in the next parts of task 1.19
//! (RULES.md BL-SEC-7). It must not know about content files, templates or
//! SQL.

mod event;
mod signature;

pub use event::{Event, ParseEventError};
pub use signature::{SIGNATURE_HEADER, Signature, SigningKey, SigningKeyError, sign};
