//! The files every page links, built once: the stylesheet from the theme,
//! and the reader script from the bytes in the binary. `khalid build`
//! writes them and `khalid serve` answers them.

use std::path::Path;
use std::sync::LazyLock;

use khalid_core::{Config, UserMessage};
use khalid_render::{
    Asset, Feature, Features, ScriptFile, ScriptTag, Tokens, reader_script, reader_script_file,
    stylesheet, stylesheet_text,
};
use khalid_server::{MediaType, Page, PagePath};

use crate::build_message;

/// The folder, under the mount and in the output folder, that holds them.
pub(crate) const FOLDER: &str = "assets/";

/// The reader features the reader script improves. With all of them off,
/// pages link no script at all.
const SCRIPTED: [Feature; 5] = [
    Feature::HeadingLinks,
    Feature::EnlargeImages,
    Feature::SearchShortcut,
    Feature::TableOfContents,
    Feature::LinkPreviews,
];

/// The reader script's name, hash and bytes, made once per process because
/// they never change while the binary runs. Only `khalid build` makes its
/// compressed copies; `khalid serve` compresses after it starts answering,
/// so the script never delays the first page.
static READER: LazyLock<ScriptFile> = LazyLock::new(reader_script_file);

/// The site's stylesheet: the values in `theme/tokens.toml`, when the site
/// has that file, as CSS custom properties, then the shared rules.
pub(crate) fn site_stylesheet(site: &Path) -> Result<Asset, UserMessage> {
    let tokens = Tokens::load(site).map_err(|error| build_message::tokens(&error))?;
    stylesheet(&tokens).map_err(|error| build_message::render(&error))
}

/// The reader script's name, integrity value and bytes.
pub(crate) fn reader() -> &'static ScriptFile {
    &READER
}

/// The reader script with its compressed copies, for `khalid build`.
pub(crate) fn reader_with_copies() -> Result<Asset, UserMessage> {
    reader_script().map_err(|error| build_message::render(&error))
}

/// The tag pages with `features` carry for the reader script, or `None`
/// when no feature on uses it.
pub(crate) fn script_tag(config: &Config, features: &Features) -> Option<ScriptTag> {
    let used = SCRIPTED.iter().any(|&feature| features.contains(feature));
    let folder = format!("{}{FOLDER}", config.mount());
    used.then(|| ScriptTag::of_file(reader(), &folder))
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
    served(config, &name, MediaType::Css, css)
}

/// The reader script as `khalid serve` holds it, cached for a year like
/// the stylesheet.
pub(crate) fn script_page(config: &Config) -> Result<Page, UserMessage> {
    let script = reader();
    served(
        config,
        &script.name,
        MediaType::JavaScript,
        script.body.to_vec(),
    )
}

fn served(
    config: &Config,
    name: &str,
    kind: MediaType,
    body: Vec<u8>,
) -> Result<Page, UserMessage> {
    let href = format!("{}{FOLDER}{name}", config.mount());
    let path = PagePath::try_new(&href).map_err(|_| build_message::address(&href))?;
    Ok(Page::immutable(path, kind, body))
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
