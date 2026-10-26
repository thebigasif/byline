//! The Khalid linter: rules that refuse a weak post and explain why.
//!
//! This crate owns the rule engine, every rule and every message. A rule reads
//! a [`LintPost`] and the rest of the site through [`SiteView`], and each
//! finding carries a [`Message`] of three plain sentences from the
//! [`Catalogue`]: what is wrong, why it matters, what to do.
//!
//! The `SerpDad` backend runs the same rules on its own drafts, so this crate must
//! not know about files on disk, HTTP, SQL, templates or the other Khalid
//! crates. It depends on `khalid-core` only, for shared identifiers. The
//! content track builds the input model from its own types (ADR 0011).

mod catalogue;
mod lint_error;
mod lint_post;
mod lint_site;

pub use catalogue::{Catalogue, Message};
pub use lint_error::LintError;
pub use lint_post::{Heading, HeadingLevel, Image, LinkKind, LintLink, LintPost};
pub use lint_site::{GlossaryTerm, LintSite, PageRef, SiteView};
