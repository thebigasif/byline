//! The generated share image (PRD KH-SEO-2): a 1200 by 630 PNG drawn from a
//! post's title, the site name and three theme colours, in pure Rust with
//! `resvg` (ADR 0016), kept on disk by content key in [`ShareCache`]. It
//! must not know about content files, templates, themes on disk or HTTP:
//! callers hand it a [`ShareCard`].

mod render;
mod share_cache;
mod share_card;
mod share_image_error;
mod share_key;

pub use render::{HEIGHT, WIDTH, render};
pub use share_cache::{Drawn, ShareCache};
pub use share_card::{CardColors, Direction, Rgb, ShareCard};
pub use share_image_error::ShareImageError;
pub use share_key::ShareKey;
