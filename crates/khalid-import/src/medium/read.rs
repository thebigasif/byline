//! Reading the `posts/` folder of a Medium export, one HTML file per post.

use chrono::{DateTime, NaiveDateTime};
use khalid_core::AbsoluteUrl;
use scraper::{ElementRef, Html, Selector};

use crate::medium::{Author, Export, Post, PostStatus, UnreadFile};

/// Why [`read_export`] refused a folder.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExportError {
    /// No file is a Medium post, often because another folder of the
    /// export was given, such as `profile/`.
    #[error(
        "no file in the folder is a Medium post, so it is not the posts folder of a Medium export"
    )]
    NoPosts {
        /// How many files the folder had.
        files: usize,
    },
}

/// Why one file in `posts/` was not read as a post.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PostFileError {
    /// Its name does not end in `.html`.
    #[error("it is not an HTML file")]
    NotHtml,
    /// It is HTML, but has no section holding a post's body.
    #[error("it is not a Medium post: it has no body section")]
    NotAPost,
    /// Its publication time is not a time Medium writes.
    #[error("its publication time {value:?} is not a date")]
    BadDate {
        /// The time as written.
        value: String,
    },
}

/// Reads every file of a Medium export's `posts/` folder, each given as
/// its file name and its text. Posts come in the order of their file
/// names, which start with the publication date or `draft_`. A file that
/// is not a post is listed in [`Export::unread`] with the reason, so
/// nothing is skipped silently.
///
/// # Errors
///
/// Returns [`ExportError::NoPosts`] when no file is a Medium post.
pub fn read_export<N, T>(files: impl IntoIterator<Item = (N, T)>) -> Result<Export, ExportError>
where
    N: AsRef<str>,
    T: AsRef<str>,
{
    let mut files: Vec<(N, T)> = files.into_iter().collect();
    files.sort_by(|(a, _), (b, _)| a.as_ref().cmp(b.as_ref()));
    let mut export = Export::default();
    for (name, text) in &files {
        let file = name.as_ref().to_owned();
        match read_post(&file, text.as_ref()) {
            Ok(post) => export.posts.push(post),
            Err(error) => export.unread.push(UnreadFile { file, error }),
        }
    }
    if export.posts.is_empty() {
        return Err(ExportError::NoPosts { files: files.len() });
    }
    Ok(export)
}

/// The post in the file named `file`. The fields are the microformats
/// Medium marks them with: `p-name`, `p-summary`, `e-content`,
/// `dt-published`, `p-canonical` and `p-author`.
fn read_post(file: &str, text: &str) -> Result<Post, PostFileError> {
    let stem = file.strip_suffix(".html").ok_or(PostFileError::NotHtml)?;
    let page = Html::parse_document(text);
    let body = first(&page, r#"section[data-field="body"]"#).ok_or(PostFileError::NotAPost)?;
    let title = first(&page, "h1.p-name")
        .or_else(|| first(&page, "title"))
        .map(text_of)
        .unwrap_or_default();
    let subtitle = first(&page, r#"section[data-field="subtitle"]"#)
        .map(text_of)
        .filter(|subtitle| !subtitle.is_empty());
    let published = first(&page, "footer time.dt-published")
        .and_then(|time| time.value().attr("datetime"))
        .map(utc)
        .transpose()?;
    let author = first(&page, "footer a.p-author")
        .map(|link| Author {
            name: text_of(link),
            profile: address(link),
        })
        .filter(|author| !author.name.is_empty());
    Ok(Post {
        id: id_of(stem).to_owned(),
        file: file.to_owned(),
        title,
        subtitle,
        body: body.inner_html(),
        status: match published {
            Some(_) => PostStatus::Published,
            None => PostStatus::Draft,
        },
        published,
        canonical: first(&page, "footer a.p-canonical").and_then(address),
        author,
    })
}

/// The first element `css` selects in `page`.
fn first<'a>(page: &'a Html, css: &str) -> Option<ElementRef<'a>> {
    let selector = Selector::parse(css).ok()?;
    page.select(&selector).next()
}

/// The element's text, with every run of spaces and line breaks made one
/// space.
fn text_of(element: ElementRef<'_>) -> String {
    let text: String = element.text().collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The link's address, when it is a full one.
fn address(link: ElementRef<'_>) -> Option<AbsoluteUrl> {
    AbsoluteUrl::parse(link.value().attr("href")?).ok()
}

/// A time as Medium writes it, such as `2024-03-12T09:30:44.123Z`, in UTC.
fn utc(value: &str) -> Result<NaiveDateTime, PostFileError> {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.naive_utc())
        .map_err(|_| PostFileError::BadDate {
            value: value.to_owned(),
        })
}

/// Medium's id from a file name without `.html`: the hex code after its
/// last hyphen, as in `2024-03-12_Rye-3f1a8c4e2b9d`. A name without one is
/// its own id.
fn id_of(stem: &str) -> &str {
    stem.rsplit_once('-')
        .map(|(_, id)| id)
        .filter(|id| id.len() >= 8 && id.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(stem)
}
