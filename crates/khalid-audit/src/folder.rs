//! A built site folder as a crawl source, answering as a static host would.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use khalid_core::AbsoluteUrl;
use url::Url;

use crate::{Answer, AuditError, Source, Unreached};

/// The file a static host reads its redirects from (ADR 0031).
const REDIRECTS: &str = "_redirects";
/// The folder `khalid build` puts the host's root files in, for a blog in a
/// folder.
const ROOT_FILES: &str = "_root";

/// A folder `khalid build` wrote, answering each address of the blog the
/// way a static host does: the file at its path, `index.html` for a
/// folder, the redirects in `_redirects`, and 404 for anything else. It
/// sends no headers but `Content-Type` and `Location`, so the checks on
/// HTTPS headers are left to a crawl over HTTP.
#[derive(Debug, Clone)]
pub struct Folder {
    root: PathBuf,
    mount: String,
    redirects: BTreeMap<String, (String, u16)>,
}

impl Folder {
    /// The folder at `root`, holding the blog whose home page is `home`.
    ///
    /// # Errors
    ///
    /// Returns [`AuditError::Folder`] when `root` is not a folder, and
    /// [`AuditError::Redirects`] when its `_redirects` file cannot be read.
    pub fn open(root: &Path, home: &AbsoluteUrl) -> Result<Self, AuditError> {
        if !root.is_dir() {
            return Err(AuditError::Folder {
                folder: root.to_path_buf(),
            });
        }
        let mut file = root.to_path_buf();
        file.push(REDIRECTS);
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(source) => return Err(AuditError::Redirects { file, source }),
        };
        let redirects = text
            .lines()
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                let (from, to) = (words.next()?, words.next()?);
                let status = words.next().map_or(Some(301), |code| code.parse().ok())?;
                (!from.starts_with('#')).then(|| (from.to_owned(), (to.to_owned(), status)))
            })
            .collect();
        let mount = Url::parse(home.as_str())
            .map_or_else(|_| "/".to_owned(), |home| home.path().to_owned());
        Ok(Self {
            root: root.to_path_buf(),
            mount,
            redirects,
        })
    }

    /// The answer for the path `path` from the site root, such as
    /// `/blog/why/`.
    fn answer(&self, path: &str) -> Answer {
        if let Some((to, status)) = self.redirects.get(path) {
            return Answer::new(*status, Vec::new()).with_header("location", to);
        }
        let Some(file) = self.file(path) else {
            return Answer::new(404, Vec::new());
        };
        if file.is_dir() {
            let to = format!("{path}/");
            return Answer::new(301, Vec::new()).with_header("location", &to);
        }
        match fs::read(&file) {
            Ok(body) => Answer::new(200, body).with_header("content-type", kind(&file)),
            Err(_) => Answer::new(404, Vec::new()),
        }
    }

    /// The file in the folder for `path`, or `None` when the path leaves
    /// the folder or names nothing there. A path outside the blog's folder
    /// is looked for in `_root`, where `khalid build` puts the files a
    /// blog in a folder needs at the root of its host, such as
    /// `robots.txt`.
    fn file(&self, path: &str) -> Option<PathBuf> {
        let mut file = self.root.clone();
        let inside = if let Some(inside) = path.strip_prefix(&self.mount) {
            inside
        } else {
            file.push(ROOT_FILES);
            path.strip_prefix('/')?
        };
        let decoded = percent_decode(inside)?;
        for part in Path::new(&decoded).components() {
            match part {
                Component::Normal(name) => file.push(name),
                _ => return None,
            }
        }
        if decoded.is_empty() || decoded.ends_with('/') {
            file.push("index.html");
        }
        file.exists().then_some(file)
    }
}

impl Source for Folder {
    fn fetch(&self, url: &AbsoluteUrl) -> impl Future<Output = Result<Answer, Unreached>> {
        let answer = Url::parse(url.as_str()).map_or_else(
            |_| Answer::new(404, Vec::new()),
            |url| self.answer(url.path()),
        );
        std::future::ready(Ok(answer))
    }
}

/// The content type a static host sends for `file`, by its extension.
fn kind(file: &Path) -> &'static str {
    match file.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("xml") => "application/xml",
        Some("json") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// `text` with its `%xx` escapes decoded, or `None` when they do not make
/// UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    percent_encoding::percent_decode_str(text)
        .decode_utf8()
        .ok()
        .map(std::borrow::Cow::into_owned)
}
