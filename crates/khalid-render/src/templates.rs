//! The default templates built into the binary.

use rust_embed::RustEmbed;

/// The default templates. `debug-embed` builds them in for debug builds too,
/// so development renders exactly what production renders.
#[derive(RustEmbed)]
#[folder = "templates/"]
struct Defaults;

/// One template: its name and its source.
pub(crate) struct Source {
    pub(crate) name: String,
    pub(crate) text: String,
}

/// Every default template, in name order.
pub(crate) fn defaults() -> Vec<Source> {
    let mut sources: Vec<Source> = Defaults::iter()
        .filter_map(|name| {
            let file = Defaults::get(&name)?;
            Some(Source {
                text: String::from_utf8_lossy(&file.data).into_owned(),
                name: name.into_owned(),
            })
        })
        .collect();
    sources.sort_by(|a, b| a.name.cmp(&b.name));
    sources
}
