//! The public API: the Content API under `/api/content/v1/` and the Admin
//! API under `/api/admin/v1/` (`docs/standards/api.md`, ADR 0003).
//!
//! This crate owns routes, the shared list [`query`] grammar (RULES.md
//! BL-API-5), keys, [`problem`] details and list [`pagination`]. It reaches
//! content only through traits other crates implement, so it must not know
//! about content files, markdown, SQL or templates.

mod admin_api;
mod admin_check;
mod admin_comments;
mod admin_delete;
mod admin_key;
mod admin_not_found;
mod admin_redirect_csv;
mod admin_redirects;
mod admin_webhooks;
pub mod api;
mod api_version;
pub mod comment_moderation;
pub mod content_api;
mod content_key;
pub mod content_store;
mod etag;
mod html_source;
pub mod keys;
pub mod media_api;
pub mod not_found_log;
pub mod pagination;
pub mod post_check;
pub mod post_password;
pub mod post_store;
pub mod preview_link;
pub mod problem;
pub mod publish_door;
pub mod query;
pub mod redirect_desk;
pub mod redirect_store;
pub mod resources;
pub mod schedule_store;
pub mod schema_template_store;
pub mod session_check;
pub mod unlock;
pub mod webhook_store;
