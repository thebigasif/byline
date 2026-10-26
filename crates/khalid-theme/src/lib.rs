//! Theme sync: the blog takes on the look of the main site it lives beside.
//!
//! This crate owns reading a site's home page and stylesheets, finding its
//! colours, fonts, logo and icon, writing them as theme tokens, fixing text
//! colours that fail WCAG contrast, and printing the proxy rule that mounts
//! the blog under a folder of the main site (plan sections 4.8 and 4.9).
//!
//! Every request goes through `khalid_outbound::Outbound` (RULES.md
//! BL-RUN-8). The crate must not know about content, posts or the database.

mod brand;
mod color;
mod css;
mod fetch;
mod fetch_error;
mod fonts;
mod links;
mod palette;
mod properties;
mod site_theme;
mod split;
mod token_values;
mod tokens;
mod tokens_error;

pub use brand::Brand;
pub use color::Rgb;
pub use fetch::{
    MAX_CSS_BYTES, MAX_HTML_BYTES, MAX_STYLESHEETS, Origin, SiteSource, Stylesheet, fetch_site,
};
pub use fetch_error::FetchError;
pub use fonts::{FontHost, FontSource, Typography};
pub use links::{PageStyle, SkipReason, Skipped, page_styles};
pub use palette::Palette;
pub use properties::CustomProperties;
pub use site_theme::SiteTheme;
pub use token_values::{FontStack, Length};
pub use tokens::{Colors, Fonts, Shape, ThemeTokens};
pub use tokens_error::TokensError;
