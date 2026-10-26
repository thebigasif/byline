//! The images beside a folder post, offered in sizes made by `khalid-media`
//! at `<mount>media/<key>/<width>.<ext>`, each body image drawn as a
//! `picture` whose `img` has `width`, `height`, `srcset`, `sizes` and `alt`,
//! so the page never shifts as it loads (RULES.md BL-WEB-5, KH-WEB-5).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use khalid_core::{Config, UserMessage};
use khalid_media::{DerivedImage, Format, MediaCache, MediaError, derive};
use khalid_render::{ImageCandidate, ImageSet};
use khalid_server::MediaType;

use crate::build_pages::LanguagePages;
use crate::post_files::{self, IMAGE, PostFile};

/// The folder under the mount, and in the output folder, that holds every size.
const FOLDER: &str = "media/";

/// How wide a body image is drawn: the text column, which `--measure`
/// keeps under 48rem, or the whole screen when that is narrower.
const SIZES: &str = "(min-width: 48rem) 48rem, 100vw";

/// One size of an image, and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaFile {
    /// Where `khalid-media` keeps it, in the site's `data/media/`.
    pub(crate) path: PathBuf,
    /// Where `khalid build` writes it, relative to the output folder.
    pub(crate) file: String,
    /// Where `khalid serve` answers it, such as `/blog/media/3f2a9c1e0b57d0aa/800.webp`.
    pub(crate) address: String,
}

/// Every size an image is offered in, as a page draws it.
#[derive(Debug)]
struct Picture {
    webp: ImageSet,
    fallback: ImageSet,
    width: u32,
    height: u32,
}

/// Makes the sizes of each image among `files` that the `site` folder's
/// `data/media/` lacks, off the async runtime (RULES.md BL-RUN-7), draws
/// the body images of `languages` showing one, and returns every size once.
pub(crate) fn attach(
    site: &Path,
    config: &Config,
    files: &[PostFile],
    languages: &mut [LanguagePages],
) -> Result<Vec<MediaFile>, UserMessage> {
    let cache = MediaCache::new(site);
    let mut pictures = BTreeMap::new();
    let mut sizes = BTreeMap::new();
    for file in files {
        let kind = MediaType::for_file(&file.address);
        if !matches!(kind, MediaType::Jpeg | MediaType::Png | MediaType::Webp) {
            continue;
        }
        let bytes = post_files::read(site, file)?;
        let image = derive(&bytes, &cache).map_err(|error| unmade(&file.source, &error))?;
        let Some((picture, made)) = picture(config, &image) else {
            continue;
        };
        pictures.insert(file.address.clone(), picture);
        sizes.extend(made.into_iter().map(|size| (size.address.clone(), size)));
    }
    for language in languages.iter_mut().filter(|_| !pictures.is_empty()) {
        for page in &mut language.pages {
            post_files::rewrite(page, |html| drawn(html, &pictures));
        }
    }
    Ok(sizes.into_values().collect())
}

/// The picture `image` makes, and its files. `None` without the WebP and
/// fallback files `derive` always makes.
fn picture(config: &Config, image: &DerivedImage) -> Option<(Picture, Vec<MediaFile>)> {
    let mut webp = Vec::new();
    let mut fallback = Vec::new();
    let mut made = Vec::with_capacity(image.files.len());
    for derived in &image.files {
        let file = format!("{FOLDER}{}/{}", image.key, derived.variant.file_name());
        let address = format!("{}{file}", config.mount());
        let candidate = ImageCandidate {
            url: address.clone(),
            width: derived.variant.width,
        };
        if derived.variant.format == Format::Webp {
            webp.push(candidate);
        } else {
            fallback.push(candidate);
        }
        made.push(MediaFile {
            path: derived.path.clone(),
            file,
            address,
        });
    }
    let set = |mut candidates: Vec<ImageCandidate>| {
        let first = candidates.pop()?;
        Some(ImageSet::new(first, candidates))
    };
    let picture = Picture {
        webp: set(webp)?,
        fallback: set(fallback)?,
        width: image.width,
        height: image.height,
    };
    Some((picture, made))
}

/// `html` with each image whose address is a key of `pictures` drawn as
/// its picture, keeping the image's `alt`.
fn drawn(html: &str, pictures: &BTreeMap<String, Picture>) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find(IMAGE) {
        let (before, image) = rest.split_at(at);
        out.push_str(before);
        let after = image.strip_prefix(IMAGE).unwrap_or(image);
        let shown = after
            .split_once('"')
            .and_then(|(address, _)| pictures.get(address));
        if let (Some(picture), Some((tag, tail))) = (shown, image.split_once('>')) {
            draw(&mut out, picture, alt(tag));
            rest = tail;
        } else {
            out.push_str(IMAGE);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// The `alt` of an `img` tag as written, already escaped.
fn alt(tag: &str) -> &str {
    tag.split_once(" alt=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map_or("", |(alt, _)| alt)
}

/// Writes `picture` with `alt`, in the shape of the templates' `picture` macro.
fn draw(out: &mut String, picture: &Picture, alt: &str) {
    let webp = attribute(&picture.webp.to_srcset());
    let fallback = &picture.fallback;
    let src = attribute(fallback.widest().map_or("", |widest| widest.url.as_str()));
    let srcset = attribute(&fallback.to_srcset());
    let (width, height) = (picture.width, picture.height);
    let _ = write!(
        out,
        "<picture>\n<source type=\"image/webp\" srcset=\"{webp}\" sizes=\"{SIZES}\">\n\
         <img src=\"{src}\" srcset=\"{srcset}\" sizes=\"{SIZES}\" width=\"{width}\" \
         height=\"{height}\" alt=\"{alt}\" loading=\"lazy\" decoding=\"async\">\n</picture>"
    );
}

/// `text` safe inside a double-quoted attribute. Addresses here are made
/// from the mount, a hex key and a width, and the mount may hold any text.
fn attribute(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The message when an image beside a post cannot be made into sizes.
fn unmade(path: &Path, error: &MediaError) -> UserMessage {
    UserMessage::new(
        format!(
            "Khalid could not make the sizes of {}: {error}.",
            path.display()
        ),
        "Every image is offered in sizes that fit each screen, so a broken image stops the page.",
        "Open the image in an editor and save it again as JPEG, PNG or WebP, or remove it, then try again.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHART: &str = "/blog/slow/page-weight.png";

    fn set(mount: &str, extension: &str) -> ImageSet {
        let url = format!("{mount}media/3f2a/");
        let at = |width| ImageCandidate {
            url: format!("{url}{width}.{extension}"),
            width,
        };
        ImageSet::new(at(320), vec![at(800)])
    }

    fn pictures(mount: &str) -> BTreeMap<String, Picture> {
        let picture = Picture {
            webp: set(mount, "webp"),
            fallback: set(mount, "png"),
            width: 800,
            height: 450,
        };
        BTreeMap::from([(CHART.to_owned(), picture)])
    }

    #[test]
    fn a_shown_image_becomes_a_picture_with_its_five_attributes() {
        let html = format!(
            "<p>Before</p>\n<img src=\"{CHART}\" alt=\"A bar chart\" loading=\"lazy\" decoding=\"async\" />\n<p>After</p>"
        );
        assert_eq!(
            drawn(&html, &pictures("/blog/")),
            "<p>Before</p>\n<picture>\n\
             <source type=\"image/webp\" srcset=\"/blog/media/3f2a/320.webp 320w, /blog/media/3f2a/800.webp 800w\" sizes=\"(min-width: 48rem) 48rem, 100vw\">\n\
             <img src=\"/blog/media/3f2a/800.png\" srcset=\"/blog/media/3f2a/320.png 320w, /blog/media/3f2a/800.png 800w\" sizes=\"(min-width: 48rem) 48rem, 100vw\" width=\"800\" height=\"450\" alt=\"A bar chart\" loading=\"lazy\" decoding=\"async\">\n\
             </picture>\n<p>After</p>"
        );
    }

    #[test]
    fn any_other_image_is_left_as_it_is() {
        let html = "<img src=\"/blog/slow/other.png\" alt=\"\" /><img src=\"https://cdn.example.com/a.png\" alt=\"\" />";
        assert_eq!(drawn(html, &pictures("/blog/")), html);
    }

    #[test]
    fn a_mount_with_markup_is_escaped_in_every_address() {
        let html = format!("<img src=\"{CHART}\" alt=\"\" />");
        let drawn = drawn(&html, &pictures("/a\"b&c/"));
        assert!(
            !drawn.contains("a\"b") && drawn.contains("/a&quot;b&amp;c/"),
            "{drawn}"
        );
    }
}
