//! `khalid build`: every page of the site rendered as HTML into an output
//! folder, `public/` in the site folder unless `--out` names another, with
//! the site's sitemaps, `robots.txt`, `llms.txt` and feeds beside them, and
//! each folder post's files in its page's folder.

use std::fs;
use std::path::{self, Path, PathBuf};

use khalid_content::{ContentTree, SiteContent};
use khalid_core::{Clock, Config, UserMessage};
use khalid_render::Renderer;
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{
    build_files, build_message, build_pages, build_redirects, build_seo, content_message,
    post_files, post_images, report,
};

/// The output folder inside a site when `--out` is not given.
const PUBLIC: &str = "public";

/// The JSON shape of `khalid build --json`.
#[derive(Serialize)]
struct BuildBody {
    out: String,
    pages: usize,
}

/// Renders the `site` folder, whose `config` and `content` were checked, into
/// `out`, or `public/` in the site. `clock` dates a feed with no posts.
pub(crate) fn run(
    site: &Path,
    config: &Config,
    content: &SiteContent,
    out: Option<&Path>,
    clock: &dyn Clock,
    output: Output,
) -> Exit {
    let out = out.map_or_else(|| within(site, PUBLIC), Path::to_path_buf);
    if inside(&out, &within(site, "content")) {
        return report::failure(output, &build_message::into_content(&out), Exit::Failure);
    }
    match build(site, config, content, &out, clock) {
        Ok(written) => success(output, &out, written),
        Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Renders every page into `out`, then writes the site's files beside them,
/// and says how many pages were written.
fn build(
    site: &Path,
    config: &Config,
    content: &SiteContent,
    out: &Path,
    clock: &dyn Clock,
) -> Result<usize, UserMessage> {
    let renderer = Renderer::with_theme(site).map_err(|error| build_message::render(&error))?;
    let mut languages = build_pages::pages(config, content)?;
    // The checked content keeps no assets, so the tree is read again for them.
    let tree = ContentTree::walk(site).map_err(|error| content_message::of(site, &error))?;
    let beside = post_files::attach(&tree, &mut languages);
    for file in &beside {
        copy(&post_files::on_disk(site, file)?, out, &file.file)?;
    }
    for size in post_images::attach(site, config, &beside, &mut languages)? {
        copy(&size.path, out, &size.file)?;
    }
    let seo_site = build_seo::site(config)?;
    let seo_pages = build_seo::heads(&seo_site, &mut languages)?;
    let mut written = 0;
    for language in &languages {
        for page in &language.pages {
            let html = renderer
                .render(&language.site, &page.meta, &page.body)
                .map_err(|error| build_message::render(&error))?;
            write(out, &page.file, &html)?;
            written += 1;
        }
    }
    let redirects = build_redirects::file(config, content)?;
    let files = build_files::files(&seo_site, &seo_pages, clock.now())?;
    for file in files.iter().chain(&redirects) {
        write(out, &file.file, &file.text)?;
    }
    Ok(written)
}

/// Copies the file at `from` to `relative` inside `out`, making its folders
/// first.
fn copy(from: &Path, out: &Path, relative: &str) -> Result<(), UserMessage> {
    let target = within(out, relative);
    let made = target.parent().map_or(Ok(()), fs::create_dir_all);
    made.and_then(|()| fs::copy(from, &target).map(drop))
        .map_err(|error| build_message::write(&target, &error))
}

/// Writes `text` to `relative` inside `out`, making its folders first.
fn write(out: &Path, relative: &str, text: &str) -> Result<(), UserMessage> {
    let file = within(out, relative);
    let made = file.parent().map_or(Ok(()), fs::create_dir_all);
    made.and_then(|()| fs::write(&file, text))
        .map_err(|error| build_message::write(&file, &error))
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
