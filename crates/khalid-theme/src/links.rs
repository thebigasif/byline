//! Finding the stylesheets a home page uses, in page order.

use khalid_core::AbsoluteUrl;
use scraper::{Html, Selector};
use url::Url;

/// One stylesheet a page asks for, in the order the page lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageStyle {
    /// A `<link rel="stylesheet">` on the same site, to be fetched.
    Linked(AbsoluteUrl),
    /// The text of a `<style>` element.
    Inline(String),
    /// A stylesheet link theme sync does not follow.
    Skipped(Skipped),
}

/// A stylesheet link that was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    href: String,
    reason: SkipReason,
}

impl Skipped {
    /// A link at `href`, as written or once resolved, skipped for `reason`.
    pub fn new(href: &str, reason: SkipReason) -> Self {
        Self {
            href: href.to_owned(),
            reason,
        }
    }

    /// The link as the page wrote it, or the full address once resolved.
    pub fn href(&self) -> &str {
        &self.href
    }

    /// Why it was not read.
    pub fn reason(&self) -> &SkipReason {
        &self.reason
    }
}

/// Why a stylesheet link was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkipReason {
    /// It is on another site. Theme sync follows only the site's own links.
    OtherSite,
    /// It is not an `http` or `https` address.
    NotWebAddress,
    /// The page links more stylesheets than one sync reads.
    TooMany,
    /// Reading it would pass the cap on CSS for one sync.
    TooLarge,
    /// The server answered with this status instead of the stylesheet.
    Status(u16),
    /// The request did not complete.
    Unreachable,
}

/// Lists the stylesheet links and `<style>` elements of the page `html`,
/// read from `home`, in page order.
///
/// Links resolve against `<base href>` when the page has one. A link is
/// [`PageStyle::Linked`] only when it is an `http` or `https` address on the
/// same site as `home` (ADR 0012); every other link is
/// [`PageStyle::Skipped`] with the reason.
pub fn page_styles(html: &str, home: &AbsoluteUrl) -> Vec<PageStyle> {
    let Ok(home) = Url::parse(home.as_str()) else {
        return Vec::new();
    };
    let Ok(styles) = Selector::parse("link[rel][href], style") else {
        return Vec::new();
    };
    let document = Html::parse_document(html);
    let base = base_address(&document, &home);
    document
        .select(&styles)
        .filter_map(|element| {
            if element.value().name() == "style" {
                return Some(PageStyle::Inline(element.text().collect()));
            }
            let href = element.attr("href")?;
            has_rel(element.attr("rel")?, &["stylesheet"]).then(|| link(&home, &base, href))
        })
        .collect()
}

/// The address relative links in `document` resolve against: its
/// `<base href>`, or `home` when it has none.
pub(crate) fn base_address(document: &Html, home: &Url) -> Url {
    Selector::parse("base[href]")
        .ok()
        .and_then(|base| {
            document
                .select(&base)
                .find_map(|element| home.join(element.attr("href")?).ok())
        })
        .unwrap_or_else(|| home.clone())
}

/// Whether a `rel` attribute holds one of `wanted`, in any case.
pub(crate) fn has_rel(rel: &str, wanted: &[&str]) -> bool {
    rel.split_ascii_whitespace()
        .any(|token| wanted.iter().any(|w| token.eq_ignore_ascii_case(w)))
}

/// Resolves one stylesheet link against the page's base address.
fn link(home: &Url, base: &Url, href: &str) -> PageStyle {
    let Some(url) = base
        .join(href)
        .ok()
        .and_then(|url| AbsoluteUrl::parse(url.as_str()).ok())
    else {
        return PageStyle::Skipped(Skipped::new(href, SkipReason::NotWebAddress));
    };
    if same_site(home.host_str().unwrap_or_default(), url.host()) {
        PageStyle::Linked(url)
    } else {
        PageStyle::Skipped(Skipped::new(url.as_str(), SkipReason::OtherSite))
    }
}

/// Whether `other` belongs to the site at `home`: the same host, or one of
/// its subdomains, with a leading `www.` ignored on both. So `example.com`,
/// `www.example.com` and `assets.example.com` are one site, and
/// `cdn.other.net` is not.
fn same_site(home: &str, other: &str) -> bool {
    let bare = |host: &str| host.strip_prefix("www.").unwrap_or(host).to_owned();
    let (home, other) = (bare(home), bare(other));
    other == home
        || other
            .strip_suffix(home.as_str())
            .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::same_site;

    #[test]
    fn subdomains_and_www_are_the_same_site_and_lookalikes_are_not() {
        assert!(same_site("example.com", "example.com"));
        assert!(same_site("www.example.com", "example.com"));
        assert!(same_site("example.com", "assets.example.com"));
        assert!(same_site("www.example.com", "cdn.www.example.com"));
        assert!(!same_site("example.com", "badexample.com"));
        assert!(!same_site("example.com", "example.com.evil.net"));
        assert!(!same_site("shop.example.com", "example.com"));
    }
}
