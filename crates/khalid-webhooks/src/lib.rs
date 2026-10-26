//! Webhooks: the events Khalid announces to other servers.
//!
//! This crate owns the event catalogue ([`Event`]). Signing, delivery and
//! the verifier for incoming webhooks join it in the next parts of task 1.19
//! (RULES.md BL-SEC-7). It must not know about content files, templates or
//! SQL.

mod event;

pub use event::{Event, ParseEventError};
