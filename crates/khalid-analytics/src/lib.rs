//! Server analytics with no cookies (KH-GRO-4).
//!
//! `khalid serve` counts each page a reader opens on the server, so the
//! count works with scripts blocked and sets nothing in the reader's
//! browser. It keeps no address, no user agent and no identifier: only the
//! day, the page and where the reader came from, as a [`ReferrerGroup`].
//! Readers sent by `chatgpt.com`, Perplexity, Gemini, Copilot and Claude
//! are grouped as [`ReferrerGroup::Ai`].
//!
//! This crate knows nothing about HTTP, SQL or files. The binary reads the
//! headers and `khalid-db` keeps the counts.

mod referrer;

pub use referrer::ReferrerGroup;
