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
//! `khalid-newsletter` keeps them in, and readers' comments, as the
//! [`CommentTable`] that `khalid-comments` moderates through.

mod comment_table;
mod content_index;
mod database;
mod db_error;
mod delivery_log;
mod key_table;
mod not_found_table;
mod posts;
mod rebuild;
mod redirect_rows;
mod schedule_table;
mod search_find;
mod search_index;
mod site_id;
mod site_keys;
mod site_lists;
mod subscriber_table;
mod webhook_table;

pub use comment_table::CommentTable;
pub use content_index::{ContentIndex, IndexedAuthor, IndexedPost, IndexedTerm};
pub use database::Database;
pub use db_error::DbError;
pub use delivery_log::DeliveryLog;
pub use key_table::KeyTable;
pub use not_found_table::NotFoundTable;
pub use rebuild::{PostChanges, RebuildSummary};
pub use redirect_rows::RedirectRows;
pub use schedule_table::ScheduleTable;
pub use search_find::{SearchHit, SearchResults};
pub use search_index::SearchPage;
pub use site_id::SiteId;
pub use subscriber_table::SubscriberTable;
pub use webhook_table::WebhookTable;
