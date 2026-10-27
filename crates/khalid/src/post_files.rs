//! The files beside a folder post, page or author (KH-CON-3): each is
//! copied into its page's folder by `khalid build` and answered at its
//! page's address by `khalid serve`, and a body image that names one by a
//! relative path points at that address.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs;
use std::path::{Component, Path, PathBuf};

use khalid_content::{Collection, ContentTree, Language};
use khalid_core::{Config, SitePath, UserMessage};
use khalid_render::{BodyHtml, PageBody};
use khalid_server::{MediaType, Page as Served, PagePath};

use crate::build_pages::{LanguagePages, Page};
use crate::{build_message, post_images};

/// How a body image starts, up to its address, as `khalid_content::render`
/// writes it.
pub(crate) const IMAGE: &str = "<img src=\"";

/// The file every page is written to, which no file beside it may replace.
const PAGE_FILE: &str = "index.html";

/// One file beside a content file, and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostFile {
    /// The file, relative to the site folder.
    pub(crate) source: PathBuf,
    /// Where `khalid build` writes it, relative to the output folder.
    pub(crate) file: String,
    /// Where `khalid serve` answers it, such as
    /// `/blog/fixing-slow-pages/page-weight.png`.
    pub(crate) address: String,
}

/// The files beside each folder entry of a site, by the entry's content
/// file, as a walk of the content found them: what [`attach`] needs of a
/// [`ContentTree`], without the text of every file it read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Assets(BTreeMap<PathBuf, Vec<PathBuf>>);

impl Assets {
    /// The files beside each folder entry of `tree`.
    pub(crate) fn of(tree: &ContentTree) -> Self {
        let entries = tree
            .languages()
            .iter()
            .flat_map(Language::collections)
            .flat_map(Collection::entries)
            .filter(|entry| !entry.assets().is_empty())
            .map(|entry| (entry.path().to_path_buf(), entry.assets().to_vec()));
        Self(entries.collect())
    }
}

/// Every file beside the content file of a page in `languages`, as
/// `assets` lists them, in page order. Each such page's body, and the copy
/// the feeds read, point their relative images at those files.
pub(crate) fn attach(assets: &Assets, languages: &mut [LanguagePages]) -> Vec<PostFile> {
    let mut files = Vec::new();
    for page in languages
        .iter_mut()
        .flat_map(|language| &mut language.pages)
    {
        let Some((source, found)) = page
            .source
            .as_deref()
            .and_then(|source| assets.0.get_key_value(source))
        else {
            continue;
        };
        let folder = source.parent().unwrap_or(Path::new(""));
        let beside: Vec<PostFile> = found
            .iter()
            .filter_map(|asset| post_file(page, folder, asset))
            .collect();
        relink(page, &beside);
        files.extend(beside);
    }
    files
}

/// Where `asset`, in the entry `folder`, goes for `page`. `None` for a name
/// that is not plain text, or that would replace the page itself.
fn post_file(page: &Page, folder: &Path, asset: &Path) -> Option<PostFile> {
    let parts: Option<Vec<&str>> = asset
        .strip_prefix(folder)
        .ok()?
        .components()
        .map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    let name = parts?.join("/");
    if name == PAGE_FILE {
        tracing::warn!(file = %asset.display(), "a file beside a post would replace its page, so it is left out");
        return None;
    }
    let folder = page.file.strip_suffix(PAGE_FILE).unwrap_or(&page.file);
    Some(PostFile {
        source: asset.to_path_buf(),
        file: format!("{folder}{name}"),
        address: format!("{}{name}", page.meta.path),
    })
}

/// Points every body image of `page` that names one of `files` by a path
/// relative to the page, such as `page-weight.png` or `./page-weight.png`,
/// at that file's address. An image the body names otherwise is left as it
/// is: a browser still finds it beside the page.
fn relink(page: &mut Page, files: &[PostFile]) {
    let base = page.meta.path.clone();
    rewrite(page, |html| relinked(html, &base, files));
}

/// Passes the body of `page`, when it has one written by a person, and
/// the copy of it the feeds read, through `change`.
pub(crate) fn rewrite(page: &mut Page, change: impl Fn(&str) -> String) {
    let body = match &mut page.body {
        PageBody::Post(post) => &mut post.body,
        PageBody::Page(plain) => &mut plain.body,
        PageBody::Author(author) => &mut author.bio,
        _ => return,
    };
    *body = BodyHtml::from_sanitised(change(body.as_str()));
    page.seo.content_html = change(&page.seo.content_html);
}

/// `html` with each image whose address is a file's name relative to the
/// page at `base` pointed at that file's address.
fn relinked(html: &str, base: &str, files: &[PostFile]) -> String {
    files.iter().fold(html.to_owned(), |html, file| {
        let name = file.address.strip_prefix(base).unwrap_or(&file.address);
        let to = format!("{IMAGE}{}\"", file.address);
        html.replace(&format!("{IMAGE}{name}\""), &to)
            .replace(&format!("{IMAGE}./{name}\""), &to)
    })
}

/// Where `file` is on disk inside the `site` folder, refusing a name or a
/// symlink that leads outside it (RULES.md BL-SEC-4).
pub(crate) fn on_disk(site: &Path, file: &PostFile) -> Result<PathBuf, UserMessage> {
    let relative = file.source.to_str().unwrap_or_default();
    SitePath::within(site, relative)
        .and_then(|path| path.resolve())
        .map_err(|error| unreadable(&file.source, &error))
}

/// The bytes of `file` in the `site` folder.
pub(crate) fn read(site: &Path, file: &PostFile) -> Result<Vec<u8>, UserMessage> {
    fs::read(on_disk(site, file)?).map_err(|error| unreadable(&file.source, &error))
}

/// The files beside folder posts in `assets`, as `khalid serve` answers
/// them, each at its page's address, and the sizes of their images, which
/// never change under their names.
pub(crate) fn served(
    folder: &Path,
    config: &Config,
    assets: &Assets,
    languages: &mut [LanguagePages],
) -> Result<Vec<Served>, UserMessage> {
    let address = |text: &str| PagePath::try_new(text).map_err(|_| build_message::address(text));
    let beside = attach(assets, languages);
    let mut pages = Vec::with_capacity(beside.len());
    for file in &beside {
        let kind = MediaType::for_file(&file.address);
        pages.push(Served::new(
            address(&file.address)?,
            kind,
            read(folder, file)?,
        ));
    }
    for size in post_images::attach(folder, config, &beside, languages)? {
        let kind = MediaType::for_file(&size.address);
        let bytes = fs::read(&size.path).map_err(|error| unreadable(&size.path, &error))?;
        pages.push(Served::immutable(address(&size.address)?, kind, bytes));
    }
    Ok(pages)
}

/// The message when a file beside a post cannot be read.
pub(crate) fn unreadable(path: &Path, error: &impl Display) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not read {}: {error}.", path.display()),
        "The files beside a post are published with it, and only from inside the site folder.",
        "Replace a link that leads outside the site with the file itself, or rename the file, then try again.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart() -> PostFile {
        PostFile {
            source: PathBuf::from("content/en/posts/slow/page-weight.png"),
            file: "fixing-slow-pages/page-weight.png".to_owned(),
            address: "/blog/fixing-slow-pages/page-weight.png".to_owned(),
        }
    }

    fn relinked_body(html: &str) -> String {
        relinked(html, "/blog/fixing-slow-pages/", &[chart()])
    }

    #[test]
    fn an_image_named_beside_the_page_points_at_its_address() {
        let to = "<img src=\"/blog/fixing-slow-pages/page-weight.png\" alt=\"A chart\" />";
        assert_eq!(
            relinked_body("<img src=\"page-weight.png\" alt=\"A chart\" />"),
            to
        );
        assert_eq!(
            relinked_body("<img src=\"./page-weight.png\" alt=\"A chart\" />"),
            to
        );
    }

    #[test]
    fn any_other_image_is_left_as_it_is() {
        for html in [
            "<img src=\"other.png\" alt=\"\" />",
            "<img src=\"page-weight.png.bak\" alt=\"\" />",
            "<img src=\"old/page-weight.png\" alt=\"\" />",
            "<img src=\"https://cdn.example.com/page-weight.png\" alt=\"\" />",
            "<a href=\"page-weight.png\">the chart</a>",
        ] {
            assert_eq!(relinked_body(html), html);
        }
    }
}
