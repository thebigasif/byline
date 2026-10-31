//! Server analytics with no cookies (KH-GRO-4).
//!
//! `khalid serve` counts each page a reader opens on the server, so the
//! count works with scripts blocked and sets nothing in the reader's
//! browser. It keeps no address, no user agent and no identifier: only the
//! day, the page and where the reader came from, as a [`ReferrerGroup`].
//! Readers sent by `chatgpt.com`, Perplexity, Gemini, Copilot, Claude and
//! other assistants are grouped as [`ReferrerGroup::Ai`], with the
//! [`AiTool`] that sent them.
//!
//! Requests from bots are not views. They go to the crawl log instead,
//! named by [`Crawler::recognise`], so a person can see which bots fetched
//! which page and when. A bot is known by the user agent it sends, which
//! anyone can copy: the log shows what a bot claimed to be.
//!
//! Between writes the server counts into a [`Tally`] in memory, and hands
//! it to an [`AnalyticsStore`] in batches.
//!
//! [`uncited`] lists the pages AI bots read that no AI assistant ever sent
//! a reader to. A bot's visit is never counted as a citation.
//!
//! [`Visitors`] counts the page views in a period per day, per [`Source`]
//! and per page, against the period before.
//!
//! This crate knows nothing about HTTP, SQL or files. The binary reads the
//! headers and `khalid-db` keeps the counts.

mod ai_tool;
mod ai_visitors;
mod crawler;
mod referrer;
mod store;
mod tally;
mod uncited;
mod visitors;

pub use ai_tool::AiTool;
pub use ai_visitors::{AiVisitors, PageViews, Period, ToolVisitors};
pub use crawler::{Crawler, CrawlerKind};
pub use referrer::ReferrerGroup;
pub use store::{
    AnalyticsError, AnalyticsStore, BoxFuture, CRAWL_DAYS_KEPT, MAX_CRAWL_ROWS_PER_DAY,
};
pub use tally::{Crawls, MAX_ROWS, Tally, Views};
pub use uncited::{Uncited, uncited};
pub use visitors::{DayViews, Source, SourceViews, Visitors};
