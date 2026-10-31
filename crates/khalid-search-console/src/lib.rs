//! The opt-in Google Search Console connection (KH-GRO-5, plan 5.11).
//!
//! A site whose owner adds `[search_console]` to `site.toml` reads its own
//! numbers from Google with the owner's own Google account: the clicks,
//! impressions and positions of each post, and whether Google has it in
//! its index. `SerpDad` is never involved unless the owner connects it.
//!
//! [`PageNumbers`] are what Google counted for one page in a [`Window`] of
//! days, and [`SearchConsoleError`] says why Google gave no answer, never
//! showing a secret or a token (RULES.md BL-RUN-5).
//!
//! This crate knows nothing about files, SQL or the command line.

mod error;
mod numbers;

pub use error::SearchConsoleError;
pub use numbers::{PageNumbers, Window};
