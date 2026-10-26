//! The web server for the public site.
//!
//! This crate owns answering HTTP. It holds the rendered pages in memory
//! ([`Pages`]), each at a clean [`PagePath`] with its [`MediaType`].
//!
//! It must not know about markdown, templates or the content tree. Pages
//! arrive already rendered, because the render path does no work at request
//! time (RULES.md BL-PERF-2); a request copies bytes.

mod media_type;
mod page;
mod page_path;
mod pages;
mod server_error;

pub use media_type::MediaType;
pub use page::Page;
pub use page_path::{PagePath, ParsePagePathError};
pub use pages::Pages;
pub use server_error::ServerError;
