//! The default templates built into the binary, and a theme's replacements for them.

use std::fs;
use std::io;
use std::path::Path;

use khalid_core::SitePath;
use rust_embed::RustEmbed;

use crate::RenderError;

/// Where a site keeps the templates that replace the defaults.
const THEME_TEMPLATES: &str = "theme/templates";

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

/// The templates in `site_root/theme/templates/`, in name order. A missing
/// folder means no replacements.
///
/// # Errors
///
/// Returns [`RenderError::UnknownTemplate`] for a file or folder whose name no
/// default template has, [`RenderError::ThemePath`] when a symlink leads out
/// of the site folder, and [`RenderError::ReadTemplate`] when a file cannot
/// be read.
pub(crate) fn overrides(site_root: &Path) -> Result<Vec<Source>, RenderError> {
    let folder = SitePath::within(site_root, THEME_TEMPLATES)?.resolve()?;
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(RenderError::ReadTemplate {
                path: folder,
                source,
            });
        }
    };
    let mut sources = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RenderError::ReadTemplate {
            path: folder.clone(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if Defaults::get(&name).is_none() {
            return Err(RenderError::UnknownTemplate { name });
        }
        let path = SitePath::within(site_root, &format!("{THEME_TEMPLATES}/{name}"))?.resolve()?;
        let text = fs::read_to_string(&path)
            .map_err(|source| RenderError::ReadTemplate { path, source })?;
        sources.push(Source { name, text });
    }
    sources.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(sources)
}
