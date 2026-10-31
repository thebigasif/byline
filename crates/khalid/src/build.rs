//! `khalid build`: every page of the site rendered as HTML into an output
//! folder, `public/` in the site folder unless `--out` names another, with
//! the stylesheet every page links in `assets/`, the site's sitemaps,
//! `robots.txt`, `llms.txt` and feeds beside them, and each folder post's
//! files in its page's folder. A headless site gets those files and no
//! HTML.

use std::fs;
use std::path::{self, Path, PathBuf};

use khalid_content::{ContentTree, SiteContent};
use khalid_core::{Clock, Config, UserMessage};
use khalid_lint::Linter;
use khalid_render::{Asset, Renderer, Tokens};
use khalid_routes::Routes;
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::post_files::Assets;
use crate::post_photos::Shown;
use crate::site_page::HeadNonce;
use crate::{
    assets, build_files, build_gate, build_lists, build_message, build_pages, build_redirects,
    build_seo, build_sitemap, categories, content_message, lint, lint_message, post_files,
    post_images, post_lock, report, search_page, search_static, share_images, site_briefs,
    static_mode,
};

/// The output folder inside a site when `--out` is not given.
const PUBLIC: &str = "public";

/// The JSON shape of `khalid build --json`.
#[derive(Serialize)]
struct BuildBody {
    out: String,
    pages: usize,
    /// The folder of files for the root of the host, for a blog in a folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<String>,
    /// The features a static site has off, each with what, why and fix.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    off: Vec<static_mode::Off>,
}

/// Renders the `site` folder, whose `config`, `routes` and `content` were
/// checked, into `out`, or `public/` in the site. `clock` dates a feed with
/// no posts.
pub(crate) fn run(
    site: &Path,
    (config, routes, content): (&Config, &Routes, &SiteContent),
    out: Option<&Path>,
    clock: &dyn Clock,
    output: Output,
) -> Exit {
    let out = out.map_or_else(|| within(site, PUBLIC), Path::to_path_buf);
    if inside(&out, &within(site, "content")) {
        return report::failure(output, &build_message::into_content(&out), Exit::Failure);
    }
    match into_folder(site, (config, routes, content), &out, clock, output) {
        Ok(written) => success(output, &out, written, config),
        Err(exit) => exit,
    }
}

/// Builds the checked site into `out` once the lint gate lets every post
/// through, as `khalid build` does, and says how many pages it wrote. A
/// refusal or a failure is printed in `output` and comes back as its exit.
pub(crate) fn into_folder(
    site: &Path,
    (config, routes, content): (&Config, &Routes, &SiteContent),
    out: &Path,
    clock: &dyn Clock,
    output: Output,
) -> Result<usize, Exit> {
    let linter = match lint::lint_settings(site).map(|settings| Linter::new(&settings, clock)) {
        Ok(Ok(linter)) => linter,
        Ok(Err(error)) => {
            return Err(report::failure(
                output,
                &lint_message::of(&error),
                Exit::Internal,
            ));
        }
        Err(message) => return Err(report::failure(output, &message, Exit::Failure)),
    };
    let briefs = site_briefs::load(site)
        .map_err(|message| report::failure(output, &message, Exit::Failure))?;
    let refused = build_gate::refused(&linter, content, config, routes, briefs);
    if !refused.is_empty() {
        return Err(build_gate::refuse(output, &refused));
    }
    build(site, config, routes, content, out, clock)
        .map_err(|message| report::failure(output, &message, Exit::Failure))
}

/// Renders every page into `out`, then writes the stylesheet they link and
/// the site's files beside them, and says how many pages were written. A
/// headless site gets the files only, and its theme is never read (ADR
/// "headless is a mode").
fn build(
    site: &Path,
    config: &Config,
    routes: &Routes,
    content: &SiteContent,
    out: &Path,
    clock: &dyn Clock,
) -> Result<usize, UserMessage> {
    let theme = if config.mode().has_pages() {
        let renderer = Renderer::with_theme(site).map_err(|error| build_message::render(&error))?;
        Some((renderer, assets::site_stylesheet(site)?))
    } else {
        None
    };
    let tokens = if config.mode().has_pages() {
        Tokens::load(site).map_err(|error| build_message::tokens(&error))?
    } else {
        Tokens::default()
    };
    let href = theme
        .as_ref()
        .map(|(_, css)| assets::href(config, css))
        .unwrap_or_default();
    let mut languages = build_pages::pages(config, routes, (content, &[]), &href)?;
    let renderer = theme.as_ref().map(|(renderer, _)| renderer);
    post_lock::lock(config, content, renderer, &mut languages);
    if let Some((renderer, _)) = &theme {
        search_page::add(config, renderer, &mut languages)?;
        let categories = categories::of(site)?;
        let sources = (content, categories.as_slice());
        build_lists::add((config, routes), renderer, sources, &mut languages)?;
        build_sitemap::add(config, renderer, &mut languages)?;
    }
    // The checked content keeps no assets, so the tree is read again for
    // them. A headless site gets them too: its feeds point at them.
    let tree = ContentTree::walk(site).map_err(|error| content_message::of(site, &error))?;
    let beside = post_files::attach(&Assets::of(&tree), &mut languages);
    for file in &beside {
        copy(&post_files::on_disk(site, file)?, out, &file.file)?;
    }
    let mut shown = Shown::default();
    for size in post_images::attach(site, config, &beside, &mut languages, &mut shown)? {
        copy(&size.path, out, &size.file)?;
    }
    let seo_site = build_seo::site(config)?;
    let cards = share_images::attach(site, config, &tokens, &seo_site, &mut languages)?;
    for file in share_images::draw_now(site, cards)? {
        let mut from = site.to_path_buf();
        from.push("data");
        from.push(&file);
        copy(&from, out, &file)?;
    }
    let seo_pages = build_seo::heads(&seo_site, &mut languages)?;
    build_sitemap::fill(config, &seo_site, &seo_pages, &mut languages)?;
    let mut written = 0;
    let locks = static_mode::locks(config.mode())?;
    if let Some((renderer, css)) = &theme {
        write_asset(out, css)?;
        write_fonts(site, out)?;
        for language in &languages {
            for page in &language.pages {
                let html = page.rendered(renderer, &language.site, HeadNonce::Static)?;
                let html =
                    static_mode::lock(locks.as_ref(), html, &language.site.locale, &page.file)?;
                write(out, &page.file, &html)?;
                written += 1;
            }
        }
    }
    let redirects = build_redirects::file(site, config, routes, content)?;
    let files = build_files::files(
        &seo_site,
        &seo_pages,
        config.seo().indexnow.as_ref(),
        clock.now(),
    )?;
    let search = search_static::files(config, &languages)?;
    for file in files.iter().chain(&redirects).chain(&search) {
        write(out, &file.file, &file.text)?;
    }
    for file in build_files::root_files(&seo_site)? {
        let relative = format!("{}/{}", build_files::ROOT_FOLDER, file.file);
        write(out, &relative, &file.text)?;
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

/// Copies the site's self-hosted font files, `theme/fonts/*.woff2`, into
/// `out/assets/fonts/`, where the stylesheet's `@font-face` rules point
/// (G20). Anything else in the folder stays private.
fn write_fonts(site: &Path, out: &Path) -> Result<(), UserMessage> {
    let from = {
        let mut folder = site.to_path_buf();
        folder.push("theme");
        folder.push("fonts");
        folder
    };
    let Ok(entries) = fs::read_dir(&from) else {
        return Ok(());
    };
    let into = within(out, "assets/fonts/");
    fs::create_dir_all(&into).map_err(|error| build_message::write(&into, &error))?;
    for entry in entries {
        let entry = entry.map_err(|error| build_message::write(&from, &error))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".woff2") {
            continue;
        }
        let file = within(&into, &name);
        fs::copy(entry.path(), &file).map_err(|error| build_message::write(&file, &error))?;
    }
    Ok(())
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

/// Says how many pages were written and where, or for a headless site that
/// only its files were, where the files for the host's root are, and for a
/// static site which features are off.
fn success(output: Output, out: &Path, pages: usize, config: &Config) -> Exit {
    let shown = out.display().to_string();
    let mode = config.mode();
    let off = static_mode::off(mode);
    let root = (config.mount() != "/")
        .then(|| within(out, build_files::ROOT_FOLDER).display().to_string());
    let mut said = if mode.has_pages() {
        format!(
            "Built {pages} pages in {shown}.\n{}",
            static_mode::human(&off)
        )
    } else {
        format!(
            "Built the feeds, sitemaps and redirects in {shown}. The site is headless, so Khalid writes no pages.\n"
        )
    };
    if let Some(root) = &root {
        let host = config.address().as_str().trim_end_matches('/');
        said = format!(
            "{said}Search engines read robots.txt only at {host}/robots.txt, outside the blog's folder, so its copy is in {root}. Copy the files in that folder to the root of the main site.\n"
        );
    }
    match output {
        Output::Human => report::text(&said, Exit::Success),
        Output::Json => report::json(
            &BuildBody {
                out: shown,
                pages,
                root,
                off,
            },
            Exit::Success,
        ),
    }
}
