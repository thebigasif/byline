//! What a reader's browser and the admin download first, compressed: the
//! public stylesheet and the admin's first scripts.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;

use crate::SizeError;

/// The compressed sizes, in kilobytes of 1,024 bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct Sizes {
    public_css_kb: f64,
    admin_js_kb: Option<f64>,
}

impl Sizes {
    /// Reads every stylesheet the home page in `build` links, as the size
    /// of the Brotli copy `khalid build` writes beside it, and every script
    /// `admin/dist`'s `index.html` in `admin` loads at once, gzipped at
    /// level 9 as `admin/scripts/size-budget.mjs` counts them. Scripts the
    /// admin loads later, on a route, are not counted. An `admin` folder
    /// without an `index.html` was never built and is left out.
    ///
    /// # Errors
    ///
    /// [`SizeError::Missing`] when the home page, a linked file or its
    /// Brotli copy is not there, and [`SizeError::Io`] when one cannot be
    /// read.
    pub fn measure(build: &Path, admin: &Path) -> Result<Self, SizeError> {
        let home = read(&under(build, "index.html"))?;
        let mut css = 0;
        for href in linked(&home, Linked::Stylesheet) {
            let file = found(build, href)?;
            let mut copy = file.into_os_string();
            copy.push(".br");
            css += read_bytes(&PathBuf::from(copy))?.len();
        }
        let admin_page = under(admin, "index.html");
        let admin_js_kb = if admin_page.is_file() {
            let mut js = 0;
            for src in linked(&read(&admin_page)?, Linked::Script) {
                js += gzipped(&found(admin, src)?)?;
            }
            Some(kilobytes(js))
        } else {
            None
        };
        Ok(Self {
            public_css_kb: kilobytes(css),
            admin_js_kb,
        })
    }

    /// The public stylesheets, compressed.
    pub fn public_css_kb(&self) -> f64 {
        self.public_css_kb
    }

    /// The admin's first scripts, compressed, when the admin was built.
    pub fn admin_js_kb(&self) -> Option<f64> {
        self.admin_js_kb
    }

    /// The measures as the JSON object `scripts/perf.sh` merges into
    /// `perf/current.json`, with the names `perf/baseline.json` uses.
    pub fn to_json(&self) -> String {
        let admin = self
            .admin_js_kb
            .map(|kb| format!(", \"admin_initial_js_compressed_kb\": {kb:.2}"))
            .unwrap_or_default();
        format!(
            "{{\"public_css_compressed_kb\": {:.2}{admin}}}\n",
            self.public_css_kb
        )
    }
}

/// What kind of file a page links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linked {
    /// Each `<link rel="stylesheet">`.
    Stylesheet,
    /// Each `<script src>` and `<link rel="modulepreload">`: the scripts a
    /// page loads at once.
    Script,
}

/// The addresses `html` links of one `kind`, in page order.
pub fn linked(html: &str, kind: Linked) -> Vec<&str> {
    let mut found = Vec::new();
    for tag in html.split('<').filter_map(|rest| rest.split('>').next()) {
        let link_rel = |rel| tag.starts_with("link") && attribute(tag, "rel") == Some(rel);
        let wanted = match kind {
            Linked::Stylesheet => link_rel("stylesheet"),
            Linked::Script => {
                (tag.starts_with("script") && attribute(tag, "src").is_some())
                    || link_rel("modulepreload")
            }
        };
        if let Some(address) = attribute(tag, "src").or_else(|| attribute(tag, "href"))
            && wanted
        {
            found.push(address);
        }
    }
    found
}

/// The value of `name="..."` in one tag's text.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let (_, rest) = tag.split_once(&format!(" {name}=\""))?;
    rest.split('"').next()
}

/// The file under `dir` an address names. The address starts with the
/// folder the site or admin is mounted at, which `dir` does not have, so
/// leading folders are dropped until a file is there. An address that
/// climbs out with `..` is never followed.
fn found(dir: &Path, address: &str) -> Result<PathBuf, SizeError> {
    let relative = address.trim_start_matches('/');
    let mut rest = Some(relative).filter(|path| !path.split('/').any(|part| part == ".."));
    while let Some(path) = rest {
        let file = under(dir, path);
        if file.is_file() {
            return Ok(file);
        }
        rest = path.split_once('/').map(|(_, after)| after);
    }
    Err(SizeError::Missing {
        path: under(dir, relative),
    })
}

fn under(dir: &Path, relative: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.extend(relative.split('/'));
    path
}

fn read(path: &Path) -> Result<String, SizeError> {
    let bytes = read_bytes(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, SizeError> {
    if !path.is_file() {
        return Err(SizeError::Missing {
            path: path.to_path_buf(),
        });
    }
    fs::read(path).map_err(|source| SizeError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// The length of the file at `path` gzipped at level 9.
fn gzipped(path: &Path) -> Result<usize, SizeError> {
    let bytes = read_bytes(path)?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(&bytes)
        .and_then(|()| encoder.finish())
        .map(|out| out.len())
        .map_err(|source| SizeError::Io {
            path: path.to_path_buf(),
            source,
        })
}

fn kilobytes(bytes: usize) -> f64 {
    f64::from(u32::try_from(bytes).unwrap_or(u32::MAX)) / 1024.0
}
