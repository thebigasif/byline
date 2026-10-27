//! The default templates built into the binary, and a theme's replacements for them.

use std::fs;
use std::io;
use std::path::Path;

use khalid_core::{SitePath, TemplateName};
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

/// The templates in `site_root/theme/templates/`, in name order: files that
/// replace a default, and post templates such as `post-wide.html` that a
/// post asks for with `template: wide`. A missing
/// folder means no replacements.
///
/// # Errors
///
/// Returns [`RenderError::UnknownTemplate`] for a file or folder whose name is
/// neither a post template nor one that
/// a default template has, [`RenderError::ThemePath`] when a symlink leads out
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
        if Defaults::get(&name).is_none() && !is_post_template(&name) {
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

/// Whether `name` is a post template a post may ask for, such as
/// `post-wide.html` for `template: wide`.
fn is_post_template(name: &str) -> bool {
    let inner = name
        .strip_prefix("post-")
        .and_then(|n| n.strip_suffix(".html"));
    inner.is_some_and(|inner| TemplateName::try_new(inner).is_ok())
}
