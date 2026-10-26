//! The public API: the Content API under `/api/content/v1/` and the Admin
//! API under `/api/admin/v1/` (`docs/standards/api.md`, ADR 0003).
//!
//! This crate owns routes, the shared list [`query`] grammar (RULES.md
//! BL-API-5), keys and [`problem`] details. It reaches content only through
//! traits other crates implement, so it must not know about content files,
//! markdown, SQL or templates.

pub mod problem;
pub mod query;
