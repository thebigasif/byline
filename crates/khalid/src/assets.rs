//! The files every page links, built once: the stylesheet from the theme,
//! and the reader script from the bytes in the binary. `khalid build`
//! writes them and `khalid serve` answers them.

use std::path::Path;
use std::sync::LazyLock;

use khalid_core::{Config, UserMessage};
use khalid_render::{Asset, Feature, Features, ScriptTag, Tokens, reader_script, stylesheet};
use khalid_server::{MediaType, Page, PagePath};

use crate::build_message;

/// The folder, under the mount and in the output folder, that holds them.
pub(crate) const FOLDER: &str = "assets/";

/// The reader features the reader script improves. With all of them off,
/// pages link no script at all.
const SCRIPTED: [Feature; 4] = [
    Feature::HeadingLinks,
    Feature::EnlargeImages,
    Feature::SearchShortcut,
    Feature::TableOfContents,
];

/// The reader script with its compressed copies, made once per process
/// because its bytes never change while the binary runs.
static READER: LazyLock<Option<Asset>> = LazyLock::new(|| {
    reader_script()
        .inspect_err(|error| {
            tracing::warn!(%error, "the reader script could not be compressed, so pages link none");
        })
        .ok()
});

/// The site's stylesheet: the values in `theme/tokens.toml`, when the site
/// has that file, as CSS custom properties, then the shared rules.
pub(crate) fn site_stylesheet(site: &Path) -> Result<Asset, UserMessage> {
    let tokens = Tokens::load(site).map_err(|error| build_message::tokens(&error))?;
    stylesheet(&tokens).map_err(|error| build_message::render(&error))
}

/// The reader script, or `None` in the unlikely case it could not be made,
/// when every page still reads completely.
pub(crate) fn reader() -> Option<&'static Asset> {
    READER.as_ref()
}

/// The tag pages with `features` carry for the reader script, or `None`
/// when no feature on uses it.
pub(crate) fn script_tag(config: &Config, features: &Features) -> Option<ScriptTag> {
    let used = SCRIPTED.iter().any(|&feature| features.contains(feature));
    let script = reader().filter(|_| used)?;
    Some(ScriptTag::new(
        script,
        &format!("{}{FOLDER}", config.mount()),
    ))
}

/// The address pages link `asset` at, such as `/blog/assets/site.3f2a9c1e.css`.
pub(crate) fn href(config: &Config, asset: &Asset) -> String {
    format!("{}{FOLDER}{}", config.mount(), asset.name())
}

/// The stylesheet as `khalid serve` holds it, cached for a year because
/// its name changes with its content.
pub(crate) fn stylesheet_page(config: &Config, asset: &Asset) -> Result<Page, UserMessage> {
    served(config, asset, MediaType::Css)
}

/// The reader script as `khalid serve` holds it, cached for a year like
/// the stylesheet, or `None` without one.
pub(crate) fn script_page(config: &Config) -> Result<Option<Page>, UserMessage> {
    reader()
        .map(|script| served(config, script, MediaType::JavaScript))
        .transpose()
}

fn served(config: &Config, asset: &Asset, kind: MediaType) -> Result<Page, UserMessage> {
    let href = href(config, asset);
    let path = PagePath::try_new(&href).map_err(|_| build_message::address(&href))?;
    Ok(Page::immutable(path, kind, asset.body().to_vec()))
}
