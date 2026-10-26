//! Theme sync: the blog takes on the look of the main site it lives beside.
//!
//! This crate owns reading a site's home page and stylesheets, finding its
//! colours, fonts, logo and icon, writing them as theme tokens, and fixing
//! text colours that fail WCAG contrast (plan section 4.9).
//!
//! Every request goes through `khalid_outbound::Outbound` (RULES.md
//! BL-RUN-8). The crate must not know about content, posts, templates or
//! the database.

mod color;
mod links;
mod split;

pub use color::Rgb;
pub use links::{PageStyle, SkipReason, Skipped, page_styles};
