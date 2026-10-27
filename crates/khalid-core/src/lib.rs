//! Shared building blocks for every Khalid crate.
//!
//! This crate owns the types the other crates agree on: the message a person
//! reads ([`UserMessage`]), validated identifiers such as [`Slug`], [`Locale`]
//! and [`AbsoluteUrl`], the [`Clock`], the [`Config`], the [`Secret`] wrapper,
//! [`SitePath`], and [`RobotsDirectives`], which front matter and the SEO
//! outputs share. It must not know about HTTP or SQL. It depends on no other Khalid crate, and its only
//! I/O is reading the system clock and resolving a [`SitePath`] on disk.

mod absolute_url;
mod clock;
mod config;
mod locale;
mod robots;
mod secret;
mod seo_settings;
mod site_path;
mod slug;
mod slug_from_title;
pub mod toml_location;
mod user_message;

pub use absolute_url::{AbsoluteUrl, ParseUrlError};
#[cfg(any(test, feature = "test-util"))]
pub use clock::FixedClock;
pub use clock::{Clock, SystemClock};
pub use config::{Config, ConfigError, Environment, Mode, SlugMode};
pub use locale::{Locale, ParseLocaleError};
pub use robots::{
    Following, ImagePreview, Indexing, ParseRobotsError, PreviewLimit, Restriction,
    RobotsDirectives,
};
pub use secret::Secret;
pub use seo_settings::{ArchiveIndex, SeoSettings};
pub use site_path::{SitePath, SitePathError};
pub use slug::{ParseSlugError, Slug};
pub use user_message::UserMessage;
