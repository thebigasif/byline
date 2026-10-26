//! How a site's addresses are shaped (plan section 4.11).
//!
//! This crate owns `routes.toml`: collections and their permalink patterns,
//! channels and their filters, and the prefixes of tag and category pages.
//! A site without the file gets the defaults. It must not know about content
//! files, templates or HTTP; callers hand it a post's slug, date and
//! collection and get an address back.

mod address;
mod channel;
mod permalink;
mod routes;
mod routes_error;

pub use address::{AddressError, PostAddress, Redirect};
pub use channel::{Channel, ChannelFilter};
pub use permalink::Permalink;
pub use routes::{Collection, Routes};
pub use routes_error::RoutesError;
