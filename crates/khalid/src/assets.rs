//! The files every page links, built once from the theme: today, the one
//! stylesheet. `khalid build` writes them and `khalid serve` answers them.

use std::path::Path;

use khalid_core::{Config, UserMessage};
use khalid_render::{Asset, Tokens, stylesheet, stylesheet_text};
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

/// The stylesheet as `khalid serve` holds it from `tokens`, cached for a
/// year because its name changes with its content. It is the file
/// [`site_stylesheet`] builds, at the same address, made without the
/// compressed copies: the server compresses its pages after it answers,
/// and Brotli's best setting would cost the start tens of megabytes.
pub(crate) fn stylesheet_page(config: &Config, tokens: &Tokens) -> Result<Page, UserMessage> {
    let css = stylesheet_text(tokens).into_bytes();
    let name = Asset::hashed_name("site", "css", &css);
    let href = format!("{}{FOLDER}{name}", config.mount());
    let path = PagePath::try_new(&href).map_err(|_| build_message::address(&href))?;
    Ok(Page::immutable(path, MediaType::Css, css))
}

#[cfg(test)]
mod tests {
    use khalid_core::Environment;

    use super::*;

    #[test]
    fn the_served_stylesheet_is_the_built_one_at_the_same_address() {
        let text = "name = \"a\"\naddress = \"https://www.example.com\"\n\
                    default_language = \"en\"\nlanguages = [\"en\"]\n";
        let config =
            Config::from_toml(Path::new("site.toml"), text, &Environment::empty()).unwrap();
        let tokens = Tokens::from_toml("[colors]\nlink = \"#b3261e\"").unwrap();
        let built = stylesheet(&tokens).unwrap();
        let page = stylesheet_page(&config, &tokens).unwrap();
        assert_eq!(page.path().as_str(), href(&config, &built));
        let folder = format!("{}{FOLDER}site.", config.mount());
        assert!(page.path().as_str().starts_with(&folder));
        assert_eq!(page.body().unwrap().as_ref(), built.body());
    }
}
