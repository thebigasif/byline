//! The public API: the Content API under `/api/content/v1/` and the Admin
//! API under `/api/admin/v1/` (`docs/standards/api.md`, ADR 0003).
//!
//! This crate owns routes, the shared list [`query`] grammar (RULES.md
//! BL-API-5), keys, [`problem`] details and list [`pagination`]. It reaches
//! content only through traits other crates implement, so it must not know
//! about content files, markdown, SQL or templates.

mod admin_api;
mod admin_key;
pub mod api;
mod api_version;
pub mod content_api;
mod content_key;
pub mod content_store;
mod etag;
mod html_source;
pub mod keys;
pub mod pagination;
pub mod post_store;
pub mod preview_link;
pub mod problem;
pub mod publish_door;
pub mod query;
pub mod resources;
pub mod schedule_store;
