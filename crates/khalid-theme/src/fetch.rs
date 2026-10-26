//! Reading a site's home page and the stylesheets it links to.

use khalid_core::AbsoluteUrl;
use khalid_outbound::Outbound;

use crate::{FetchError, PageStyle, SkipReason, Skipped, page_styles};

/// The largest home page theme sync reads, 2 MiB. Real home pages are well
/// under this; a larger answer is not a page to take a theme from.
pub const MAX_HTML_BYTES: usize = 2 * 1024 * 1024;

/// The most CSS read from linked stylesheets in all, 4 MiB. A stylesheet
/// that would pass it is skipped.
pub const MAX_CSS_BYTES: usize = 4 * 1024 * 1024;

/// The most linked stylesheets fetched for one sync. Later links are skipped.
pub const MAX_STYLESHEETS: usize = 16;

/// A site's home page and the CSS it uses, kept raw for the extraction pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteSource {
    home: AbsoluteUrl,
    html: String,
    stylesheets: Vec<Stylesheet>,
    skipped: Vec<Skipped>,
}

impl SiteSource {
    /// The address the home page was read from.
    pub fn home(&self) -> &AbsoluteUrl {
        &self.home
    }

    /// The home page's HTML as sent.
    pub fn html(&self) -> &str {
        &self.html
    }

    /// The CSS that was read, in the order the page lists it.
    pub fn stylesheets(&self) -> &[Stylesheet] {
        &self.stylesheets
    }

    /// The stylesheet links that were not read, and why.
    pub fn skipped(&self) -> &[Skipped] {
        &self.skipped
    }
}

/// One stylesheet's text and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stylesheet {
    origin: Origin,
    css: String,
}

impl Stylesheet {
    /// Where the CSS came from.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// The CSS as sent.
    pub fn css(&self) -> &str {
        &self.css
    }
}

/// Where a stylesheet came from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Origin {
    /// A `<link rel="stylesheet">` on the same site, at this address.
    Linked(AbsoluteUrl),
    /// A `<style>` element in the home page.
    Inline,
}

/// Reads the home page at `home`, then every same-site stylesheet it links,
/// each through `outbound` and within the caps above.
///
/// A stylesheet that fails is listed in [`SiteSource::skipped`] and the
/// sync goes on; only a failure to read the home page stops it.
///
/// # Errors
///
/// Returns [`FetchError::Unreachable`] when the home page request fails,
/// including a private address the guard refused; [`FetchError::Status`]
/// for an answer other than 2xx; and [`FetchError::TooLarge`] for a page
/// over [`MAX_HTML_BYTES`].
pub async fn fetch_site(outbound: &Outbound, home: &AbsoluteUrl) -> Result<SiteSource, FetchError> {
    let page = outbound
        .get(home)
        .await
        .map_err(|source| FetchError::Unreachable { source })?;
    if !(200..300).contains(&page.status()) {
        return Err(FetchError::Status {
            status: page.status(),
        });
    }
    if page.body().len() > MAX_HTML_BYTES {
        return Err(FetchError::TooLarge {
            limit: MAX_HTML_BYTES,
        });
    }
    let html = String::from_utf8_lossy(page.body()).into_owned();
    let mut read = Read::default();
    for style in page_styles(&html, home) {
        match style {
            PageStyle::Inline(css) => read.stylesheets.push(Stylesheet {
                origin: Origin::Inline,
                css,
            }),
            PageStyle::Skipped(skipped) => read.skipped.push(skipped),
            PageStyle::Linked(url) => read.linked(outbound, url).await,
        }
    }
    Ok(SiteSource {
        home: home.clone(),
        html,
        stylesheets: read.stylesheets,
        skipped: read.skipped,
    })
}

/// What one sync has read so far.
#[derive(Default)]
struct Read {
    stylesheets: Vec<Stylesheet>,
    skipped: Vec<Skipped>,
    linked: usize,
    css_bytes: usize,
}

impl Read {
    /// Fetches one linked stylesheet, or records why not.
    async fn linked(&mut self, outbound: &Outbound, url: AbsoluteUrl) {
        let reason = if self.linked >= MAX_STYLESHEETS {
            SkipReason::TooMany
        } else {
            self.linked = self.linked.saturating_add(1);
            match outbound.get(&url).await {
                Ok(sheet) if !(200..300).contains(&sheet.status()) => {
                    SkipReason::Status(sheet.status())
                }
                Ok(sheet) if self.css_bytes.saturating_add(sheet.body().len()) > MAX_CSS_BYTES => {
                    SkipReason::TooLarge
                }
                Ok(sheet) => {
                    self.css_bytes = self.css_bytes.saturating_add(sheet.body().len());
                    self.stylesheets.push(Stylesheet {
                        origin: Origin::Linked(url),
                        css: String::from_utf8_lossy(sheet.body()).into_owned(),
                    });
                    return;
                }
                Err(_) => SkipReason::Unreachable,
            }
        };
        self.skipped.push(Skipped::new(url.as_str(), reason));
    }
}
