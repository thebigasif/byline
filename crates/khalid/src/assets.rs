//! The files every page links, built once from the theme: today, the one
//! stylesheet. `khalid build` writes them and `khalid serve` answers them.

use std::path::Path;

use khalid_core::{Config, UserMessage};
use khalid_render::{Asset, Tokens, stylesheet};
use khalid_server::{MediaType, Page, PagePath};

use crate::build_message;

/// The folder, under the mount and in the output folder, that holds them.
pub(crate) const FOLDER: &str = "assets/";

/// The site's stylesheet: the values in `theme/tokens.toml`, when the site
/// has that file, as CSS custom properties, then the shared rules.
pub(crate) fn site_stylesheet(site: &Path) -> Result<Asset, UserMessage> {
    let tokens = Tokens::load(site).map_err(|error| build_message::tokens(&error))?;
    stylesheet(&tokens).map_err(|error| build_message::render(&error))
}

/// The address pages link `asset` at, such as `/blog/assets/site.3f2a9c1e.css`.
pub(crate) fn href(config: &Config, asset: &Asset) -> String {
    format!("{}{FOLDER}{}", config.mount(), asset.name())
}

/// The stylesheet as `khalid serve` holds it, cached for a year because
/// its name changes with its content.
pub(crate) fn stylesheet_page(config: &Config, asset: &Asset) -> Result<Page, UserMessage> {
    let href = href(config, asset);
    let path = PagePath::try_new(&href).map_err(|_| build_message::address(&href))?;
    Ok(Page::immutable(path, MediaType::Css, asset.body().to_vec()))
}
