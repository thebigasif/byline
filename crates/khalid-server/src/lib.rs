//! The web server for the public site.
//!
//! This crate owns answering HTTP. It holds the rendered pages in memory
//! ([`Pages`]), each at a clean [`PagePath`] with its [`MediaType`], and the
//! [`router`] serves them, redirecting the addresses that differ only by a
//! trailing slash or `index.html`. Every response carries the
//! [`SecurityHeaders`], with a Content Security Policy whose nonce is new
//! for each request (BL-SEC-2). [`serve`] answers until told to stop, then
//! stops gracefully (BL-RUN-10).
//!
//! It must not know about markdown, templates or the content tree. Pages
//! arrive already rendered, because the render path does no work at request
//! time (RULES.md BL-PERF-2); a request copies bytes.

mod clean_url;
mod compress;
mod etag;
mod live_pages;
mod media_type;
mod page;
mod page_path;
mod pages;
mod render;
mod respond;
mod security;
mod serve;
mod server_error;

pub use live_pages::LivePages;
pub use media_type::MediaType;
pub use page::Page;
pub use page_path::{PagePath, ParsePagePathError};
pub use pages::Pages;
pub use render::{Render, Rendered, render_pages};
pub use respond::router;
pub use security::SecurityHeaders;
pub use serve::{Stopped, serve};
pub use server_error::ServerError;
