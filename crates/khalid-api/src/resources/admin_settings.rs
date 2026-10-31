//! The site's core settings as the Admin API reads and changes them.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The site's core settings from `site.toml`, in Admin API shape. The
/// running server reads `site.toml` once at start, so a change applies
/// after a restart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminSettings {
    /// The site's name.
    pub name: String,
    /// The address of the main site, such as `https://www.example.com`.
    pub address: String,
    /// The path the blog lives under, such as `/blog/`.
    pub mount: String,
    /// The language a reader gets first, such as `en`.
    pub default_language: String,
    /// Every language the site is written in.
    pub languages: Vec<String>,
    /// When `site.toml` last changed. A change sends it back.
    pub updated_at: DateTime<Utc>,
    /// Whether `site.toml` says something the running server does not use
    /// yet, so a restart is needed to apply it.
    pub restart_needed: bool,
}

/// A change to the core settings: only the fields sent change.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SettingsChange {
    /// A new name.
    pub name: Option<String>,
    /// A new address of the main site.
    pub address: Option<String>,
    /// A new path for the blog, such as `/blog/`.
    pub mount: Option<String>,
    /// A new default language, which `languages` must include.
    pub default_language: Option<String>,
    /// A new list of languages.
    pub languages: Option<Vec<String>>,
    /// The `updated_at` of the copy this change was made on.
    pub updated_at: DateTime<Utc>,
}
