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

mod dates;
mod direction;
mod escape;
mod image;
mod listing;
mod page_body;
mod page_meta;
mod post;
mod render_error;
mod renderer;
mod serde_display;
mod site;
mod strings;
mod templates;

pub use image::{ImageCandidate, ImageSet, Loading, RenderImage};
pub use listing::{Pagination, PostCard, RenderListing};
pub use page_body::PageBody;
pub use page_meta::{Alternate, PageKind, PageMeta, Robots, ShareImage};
pub use post::{BodyHtml, Link, RenderPost};
pub use render_error::RenderError;
pub use renderer::Renderer;
pub use site::{Nav, NavLink, RenderSite};
