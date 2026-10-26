//! `khalid build`: every page of the site rendered as HTML into an output
//! folder, `public/` in the site folder unless `--out` names another, with
//! the stylesheet every page links in `assets/`.

use std::fs;
use std::path::{self, Path, PathBuf};

use khalid_content::SiteContent;
use khalid_core::{Config, UserMessage};
use khalid_render::{Asset, Renderer};
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{assets, build_message, build_pages, report};

/// The output folder inside a site when `--out` is not given.
const PUBLIC: &str = "public";

/// The JSON shape of `khalid build --json`.
#[derive(Serialize)]
struct BuildBody {
    out: String,
    pages: usize,
}

/// Renders the `site` folder, whose `config` and `content` were checked, into
/// `out`, or `public/` in the site.
pub(crate) fn run(
    site: &Path,
    config: &Config,
    content: &SiteContent,
    out: Option<&Path>,
    output: Output,
) -> Exit {
    let out = out.map_or_else(|| within(site, PUBLIC), Path::to_path_buf);
    if inside(&out, &within(site, "content")) {
        return report::failure(output, &build_message::into_content(&out), Exit::Failure);
    }
    let renderer = match Renderer::with_theme(site) {
        Ok(renderer) => renderer,
        Err(error) => {
            return report::failure(output, &build_message::render(&error), Exit::Failure);
        }
    };
    let css = match assets::site_stylesheet(site) {
        Ok(css) => css,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let languages = match build_pages::pages(config, content, &assets::href(config, &css)) {
        Ok(languages) => languages,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    if let Err(message) = write_asset(&out, &css) {
        return report::failure(output, &message, Exit::Failure);
    }
    let mut written = 0;
    for language in &languages {
        for page in &language.pages {
            let html = match renderer.render(&language.site, &page.meta, &page.body) {
                Ok(html) => html,
                Err(error) => {
                    return report::failure(output, &build_message::render(&error), Exit::Failure);
                }
            };
            let file = within(&out, &page.file);
            let made = file.parent().map_or(Ok(()), fs::create_dir_all);
            if let Err(error) = made.and_then(|()| fs::write(&file, html)) {
                return report::failure(
                    output,
                    &build_message::write(&file, &error),
                    Exit::Failure,
                );
            }
            written += 1;
        }
    }
    success(output, &out, written)
}

/// Writes `asset` into `out/assets/` with its Brotli and gzip copies beside
/// it, `.br` and `.gz`, so a web server can send them as they are.
fn write_asset(out: &Path, asset: &Asset) -> Result<(), UserMessage> {
    let folder = within(out, assets::FOLDER);
    let name = asset.name();
    let copies = [
        (name.to_owned(), asset.body()),
        (format!("{name}.br"), asset.brotli()),
        (format!("{name}.gz"), asset.gzip()),
    ];
    fs::create_dir_all(&folder).map_err(|error| build_message::write(&folder, &error))?;
    for (file, bytes) in copies {
        let file = within(&folder, &file);
        fs::write(&file, bytes).map_err(|error| build_message::write(&file, &error))?;
    }
    Ok(())
}

/// `relative` inside `folder`. Every `relative` here is a fixed name, or a
/// path made from a checked slug and language, or an asset's hashed name.
fn within(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(relative);
    path
}

/// Whether `path` is `folder` or inside it, compared as absolute paths.
fn inside(path: &Path, folder: &Path) -> bool {
    match (path::absolute(path), path::absolute(folder)) {
        (Ok(path), Ok(folder)) => path.starts_with(folder),
        _ => false,
    }
}

/// Says how many pages were written and where.
fn success(output: Output, out: &Path, pages: usize) -> Exit {
    let shown = out.display().to_string();
    match output {
        Output::Human => report::text(&format!("Built {pages} pages in {shown}.\n"), Exit::Success),
        Output::Json => report::json(&BuildBody { out: shown, pages }, Exit::Success),
    }
}
