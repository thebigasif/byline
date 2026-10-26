//! `khalid build`: every page of the site rendered as HTML into an output
//! folder, `public/` in the site folder unless `--out` names another.

use std::fs;
use std::path::{self, Path, PathBuf};

use khalid_content::SiteContent;
use khalid_core::{Config, SystemClock};
use khalid_lint::Linter;
use khalid_render::Renderer;
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{build_gate, build_message, build_pages, lint, lint_message, report};

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
    let linter = match lint::settings(site).map(|settings| Linter::new(&settings, &SystemClock)) {
        Ok(Ok(linter)) => linter,
        Ok(Err(error)) => {
            return report::failure(output, &lint_message::of(&error), Exit::Internal);
        }
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let refused = build_gate::refused(&linter, content, config);
    if !refused.is_empty() {
        return build_gate::refuse(output, &refused);
    }
    let renderer = match Renderer::with_theme(site) {
        Ok(renderer) => renderer,
        Err(error) => {
            return report::failure(output, &build_message::render(&error), Exit::Failure);
        }
    };
    let languages = match build_pages::pages(config, content) {
        Ok(languages) => languages,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
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

/// `relative` inside `folder`. Every `relative` here is a fixed name, or a
/// path made from a checked slug and language.
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
