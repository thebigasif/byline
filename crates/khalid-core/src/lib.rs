//! Shared building blocks for every Khalid crate.
//!
//! This crate owns the types the other crates agree on: the message a person
//! reads ([`UserMessage`]), validated identifiers such as [`Slug`], [`Locale`]
//! and [`AbsoluteUrl`], the [`Clock`], the [`Config`], the [`Secret`] wrapper,
//! [`SitePath`], [`RobotsDirectives`], which front matter and the SEO
//! outputs share, the [`IndexNowKey`] that settings and the SEO pings
//! share, and a post's [`PostAccess`], [`TemplateName`], [`PasswordHash`]
//! and checked [`HeadCode`], which front matter, the Admin API and the
//! server share. It must not know about HTTP or SQL. It depends on no other Khalid crate, and its only
//! I/O is reading the system clock and resolving a [`SitePath`] on disk.

mod absolute_url;
mod clock;
mod config;
mod head_code;
mod indexnow_key;
mod link_settings;
mod locale;
mod media_settings;
mod post_access;
mod reader;
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
pub use head_code::{HeadCode, HeadCodeError};
pub use indexnow_key::{IndexNowKey, ParseIndexNowKeyError};
pub use link_settings::{DomainName, ExternalRel, LinkOpening, LinkSettings, ParseDomainNameError};
pub use locale::{Locale, ParseLocaleError};
pub use media_settings::{MediaSettings, S3Keys, S3Settings};
pub use post_access::{
    ParsePasswordHashError, ParseTemplateNameError, PasswordHash, PostAccess, TemplateName,
};
pub use reader::{HomeSection, Reader};
pub use robots::{
    Following, ImagePreview, Indexing, ParseRobotsError, PreviewLimit, Restriction,
    RobotsDirectives,
};
pub use secret::Secret;
pub use seo_settings::{ArchiveIndex, IndexNowSettings, SeoSettings};
pub use site_path::{SitePath, SitePathError};
pub use slug::{ParseSlugError, Slug};
pub use user_message::UserMessage;
