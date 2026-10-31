//! The web server for the public site.
//!
//! This crate owns answering HTTP. It holds the rendered pages in memory
//! ([`Pages`]), each at a clean [`PagePath`] with its [`MediaType`], and the
//! [`router`] serves them, redirecting the addresses that differ only by a
//! trailing slash or `index.html`, and sending each [`Moved`] address on
//! with 301. Every response carries the
//! [`SecurityHeaders`], with a Content Security Policy whose nonce is new
//! for each request (BL-SEC-2). On a site whose own code waits for
//! consent, [`router_with_consent`] answers through [`chosen`], which makes
//! each page for the reader's [`Choice`], with the [`HeldCode`] only for a
//! reader who agreed. [`serve`] answers until told to stop, then
//! stops gracefully (BL-RUN-10). Beside the requests it runs two
//! [`Background`] jobs: [`watch`], which renders the pages again when a file
//! changes, and [`retry_deliveries`], which sends due webhooks again.
//!
//! It must not know about markdown, templates or the content tree. Pages
//! arrive already rendered, because the render path does no work at request
//! time (RULES.md BL-PERF-2); a request copies bytes.

mod background;
mod clean_url;
mod compress;
mod consent;
mod etag;
mod live_pages;
mod media_type;
mod moved;
mod page;
mod page_path;
mod pages;
mod render;
mod respond;
mod retry;
mod security;
mod serve;
mod server_error;
mod watch;
mod watch_events;

pub use background::Background;
pub use consent::{
    ASK_END, ASK_START, CHOSEN_END, CHOSEN_START, CONSENT_COOKIE, Choice, HELD_END, HELD_START,
    HeldCode, SITE_BODY_SLOT, SITE_HEAD_SLOT, chosen,
};
pub use live_pages::{LivePages, Replacements};
pub use media_type::MediaType;
pub use moved::Moved;
pub use page::{NONCE_SLOT, Page};
pub use page_path::{PagePath, ParsePagePathError};
pub use pages::Pages;
pub use render::{Render, Rendered, render_pages};
pub use respond::{router, router_with_consent};
pub use retry::retry_deliveries;
pub use security::SecurityHeaders;
pub use serve::{Stopped, serve};
pub use server_error::ServerError;
pub use watch::{FolderLook, poll, watch};
