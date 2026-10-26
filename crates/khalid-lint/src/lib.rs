//! The Khalid linter: rules that refuse a weak post and explain why.
//!
//! This crate owns the rule engine, every rule and every message. A rule reads
//! a [`LintPost`] and the rest of the site through [`SiteView`], and each
//! finding carries a [`Message`] of three plain sentences from the catalogue
//! file: what is wrong, why it matters, what to do. Severities come from
//! `lint.toml` through [`LintConfig`]; a [`Severity::Block`] finding stops a
//! post from being published.
//!
//! The `SerpDad` backend runs the same rules on its own drafts, so this crate must
//! not know about files on disk, HTTP, SQL, templates or the other Khalid
//! crates. It depends on `khalid-core` only, for shared identifiers. The
//! content track builds the input model from its own types (ADR 0011).

mod address;
mod blocking;
mod builtin;
mod catalogue;
mod finding;
mod gate;
mod german;
mod link_graph;
mod link_text;
mod lint_config;
mod lint_error;
mod lint_post;
mod lint_site;
mod linter;
mod rule;
mod severity;
mod site_content;
mod site_links;
mod snippet;
mod structure;
mod thresholds;
mod topics;
mod words;

pub use address::flags_slug_part;
pub use catalogue::{Catalogue, Message};
pub use finding::{Finding, Location};
pub use gate::Verdict;
pub use lint_config::LintConfig;
pub use lint_error::LintError;
pub use lint_post::{
    Cluster, ClusterRole, Heading, HeadingLevel, Image, LinkKind, LintLink, LintPost,
};
pub use lint_site::{GlossaryTerm, LintSite, PageRef, SiteView};
pub use linter::{Linter, Report};
pub use rule::{Hit, Rule, RuleId, SiteRule};
pub use severity::Severity;
pub use thresholds::Thresholds;
