//! The opt-in Google Search Console connection (KH-GRO-5, plan 5.11).
//!
//! A site whose owner adds `[search_console]` to `site.toml` reads its own
//! numbers from Google with the owner's own Google account: the clicks,
//! impressions and positions of each post, whether Google has it in its
//! index, and the long, conversational searches a post's headings do not
//! answer yet ([`follow_ups`], GEO M3). `SerpDad` is never involved unless the owner connects it.
//!
//! [`SearchConsole`] signs in with the owner's refresh token and hands back
//! a [`Session`] that asks Google, always through the one `Outbound`
//! client (RULES.md BL-RUN-8). Neither type, nor any error, ever shows a
//! secret or a token (BL-RUN-5).
//!
//! This crate knows nothing about files, SQL or the command line.

mod client;
mod error;
mod follow_ups;
mod index_status;
mod numbers;
mod trends;

pub use client::{Credentials, SearchConsole, Session};
pub use error::SearchConsoleError;
pub use follow_ups::{FOLLOW_UP_WORDS, Search, answered_by, follow_ups};
pub use index_status::{IndexStatus, Verdict};
pub use numbers::{PageNumbers, Window};
pub use trends::{Change, DECAY_MONTHS, Decay, Movers, Question, decaying, go_deeper, movers};
