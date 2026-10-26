//! The public site's templates.
//!
//! This crate owns the sandboxed `MiniJinja` environment, the default templates
//! built into the binary, a theme's replacements for them from
//! `theme/templates/`, and the words the templates show, one strings file per
//! language. It takes plain input records ([`RenderSite`], [`PageMeta`],
//! [`PageBody`]) and returns HTML.
//!
//! It must not know about markdown, the content tree, SQL or HTTP. The wiring
//! code turns content into these records; the server decides when to render
//! and caches the result.

// Unit tests use these three now; the renderer, the next part of task 1.11,
// uses them in the library.
#[cfg_attr(not(test), expect(dead_code, reason = "the renderer uses it next"))]
mod direction;
#[cfg_attr(not(test), expect(dead_code, reason = "the renderer uses it next"))]
mod escape;
mod page_body;
mod page_meta;
mod render_error;
mod serde_display;
mod site;
#[cfg_attr(not(test), expect(dead_code, reason = "the renderer uses it next"))]
mod strings;

pub use page_body::PageBody;
pub use page_meta::{Alternate, PageKind, PageMeta, Robots, ShareImage};
pub use render_error::RenderError;
pub use site::{Nav, NavLink, RenderSite};
