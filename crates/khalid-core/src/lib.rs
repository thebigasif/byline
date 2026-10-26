//! Shared building blocks for every Khalid crate.
//!
//! This crate owns the types the other crates agree on: the message a person
//! reads ([`UserMessage`]), validated identifiers such as [`Slug`], [`Locale`]
//! and [`AbsoluteUrl`], the [`Clock`], the [`Config`], the [`Secret`] wrapper
//! and [`SitePath`]. It must not know about HTTP or SQL. It depends on no other Khalid crate, and its only
//! I/O is reading the system clock and resolving a [`SitePath`] on disk.

mod absolute_url;
mod clock;
mod config;
mod locale;
mod secret;
mod site_path;
mod slug;
mod user_message;

pub use absolute_url::{AbsoluteUrl, ParseUrlError};
#[cfg(any(test, feature = "test-util"))]
pub use clock::FixedClock;
pub use clock::{Clock, SystemClock};
pub use config::Config;
pub use locale::{Locale, ParseLocaleError};
pub use secret::Secret;
pub use site_path::{SitePath, SitePathError};
pub use slug::{ParseSlugError, Slug};
pub use user_message::UserMessage;
