//! Subscribers and newsletters (plan section 5.5, KH-GRO-2, task 5.5).
//!
//! A reader signs up with a plain form, gets one email with a confirmation
//! link, and is a subscriber only once they open it (double opt-in). Every
//! email they get later carries a link that unsubscribes them in one click.
//! This crate owns those rules, the addresses and tokens they use, and the
//! store the database implements. Subscribers are visitor data:
//! a rebuild from the content files never touches them.
//!
//! It must not know about SQL, files or how mail leaves the machine. The
//! store lives in `khalid-db`, and sending goes through a transport the
//! binary picks.

mod email_address;
mod token;

pub use email_address::{EmailAddress, ParseEmailError};
pub use token::{GenerateTokenError, ParseTokenError, SubscriberToken, TokenHash};
