//! The SQLite file under `data/` (ADR 0001, RULES.md BL-FS-1).
//!
//! This crate owns the database: opening it with the settings BL-DB-1 asks
//! for, its forward-only migrations (BL-DB-2), and its typed queries, which
//! the `sqlx` macros check against the schema when the crate compiles
//! (BL-DB-3). It holds two kinds of tables, listed one per line in the
//! crate's `README.md`: derived tables, rebuilt from the content files on
//! demand, and visitor tables, which hold what people create and are never
//! rebuilt. It must not know about rendering, HTTP or the files themselves;
//! callers read the content and hand it over as a [`ContentIndex`]. It also
//! keeps the webhook delivery log, as the [`DeliveryLog`] that
//! `khalid-webhooks` writes through, and the API keys, as the [`KeyTable`]
//! that `khalid-api` checks keys against, the site's webhook list, as the
//! [`WebhookTable`] `khalid serve` announces events to, each post's
//! schedule, as the [`ScheduleTable`] the publish door keeps, the
//! newsletter subscribers, as the [`SubscriberTable`] that
//! `khalid-newsletter` keeps them in, readers' comments, as the
//! [`CommentTable`] that `khalid-comments` moderates through, and the staff
//! accounts, as the [`AccountTable`], [`SessionTable`], [`LinkTable`] and
//! [`PasskeyTable`] that `khalid-auth` signs people in with, and the
//! analytics, as the [`AnalyticsTable`] `khalid serve` counts into, the
//! visitor log, as the [`VisitorLogTable`] it writes while switched on, the
//! speed and uptime watch's looks, as the [`WatchTable`], the audit log, as
//! the [`AuditTable`] the Admin API records each change in, the `SerpDad`
//! connection, as the [`SerpdadTable`] the Admin API keeps, and the earlier
//! versions of posts, as the [`VersionTable`] the publish door keeps.

mod account_table;
mod alert_table;
mod analytics_table;
mod audit_table;
mod comment_table;
mod content_index;
mod database;
mod db_error;
mod delivery_log;
mod draft_comment_table;
mod form_table;
mod key_table;
mod link_table;
mod newsletter_sends;
mod not_found_table;
mod passkey_table;
mod posts;
mod rebuild;
mod redirect_rows;
mod schedule_table;
mod search_find;
mod search_index;
mod serpdad_table;
mod session_table;
mod site_export;
mod site_id;
mod site_keys;
mod site_lists;
mod subscriber_segments;
mod subscriber_table;
mod version_table;
mod visitor_log_table;
mod watch_table;
mod webhook_table;

pub use account_table::AccountTable;
pub use alert_table::AlertTable;
pub use analytics_table::AnalyticsTable;
pub use audit_table::AuditTable;
pub use comment_table::CommentTable;
pub use content_index::{ContentIndex, IndexedAuthor, IndexedPost, IndexedTerm};
pub use database::Database;
pub use db_error::DbError;
pub use delivery_log::DeliveryLog;
pub use draft_comment_table::DraftCommentTable;
pub use form_table::FormTable;
pub use key_table::KeyTable;
pub use link_table::LinkTable;
pub use not_found_table::NotFoundTable;
pub use passkey_table::PasskeyTable;
pub use rebuild::{PostChanges, RebuildSummary};
pub use redirect_rows::RedirectRows;
pub use schedule_table::ScheduleTable;
pub use search_find::{SearchHit, SearchResults};
pub use search_index::SearchPage;
pub use serpdad_table::SerpdadTable;
pub use session_table::SessionTable;
pub use site_export::{Cell, ExportTable, SiteExport, is_withheld_column};
pub use site_id::SiteId;
pub use subscriber_table::SubscriberTable;
pub use version_table::VersionTable;
pub use visitor_log_table::VisitorLogTable;
pub use watch_table::WatchTable;
pub use webhook_table::WebhookTable;
