//! Shared building blocks for every Khalid crate.
//!
//! This crate owns the types the other crates agree on: the message a person
//! reads ([`UserMessage`]), validated identifiers such as [`Slug`], [`Locale`] and
//! [`AbsoluteUrl`], the
//! [`Clock`], the [`Config`] and the [`Secret`] wrapper. It must not know about
//! HTTP, SQL or files. It depends on no other Khalid crate and does no I/O
//! beyond reading the system clock.

mod absolute_url;
mod clock;
mod config;
mod locale;
mod secret;
mod slug;
mod user_message;

pub use absolute_url::{AbsoluteUrl, ParseUrlError};
#[cfg(any(test, feature = "test-util"))]
pub use clock::FixedClock;
pub use clock::{Clock, SystemClock};
pub use config::Config;
pub use locale::{Locale, ParseLocaleError};
pub use secret::Secret;
pub use slug::{ParseSlugError, Slug};
pub use user_message::UserMessage;
