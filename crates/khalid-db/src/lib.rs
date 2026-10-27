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
//! `khalid-webhooks` writes through, the API keys, as the [`KeyTable`]
//! that `khalid-api` checks keys against, and the staff accounts, as the
//! [`AccountTable`], [`SessionTable`] and [`LinkTable`] that `khalid-auth`
//! signs people in with.

mod account_table;
mod content_index;
mod database;
mod db_error;
mod delivery_log;
mod key_table;
mod link_table;
mod posts;
mod rebuild;
mod session_table;
mod site_id;
mod site_lists;

pub use account_table::AccountTable;
pub use content_index::{ContentIndex, IndexedAuthor, IndexedPost, IndexedTerm};
pub use database::Database;
pub use db_error::DbError;
pub use delivery_log::DeliveryLog;
pub use key_table::KeyTable;
pub use link_table::LinkTable;
pub use rebuild::{PostChanges, RebuildSummary};
pub use session_table::SessionTable;
pub use site_id::SiteId;
