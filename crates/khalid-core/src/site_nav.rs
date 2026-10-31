//! The public site's menus and announcement bar, from the `[nav]` and
//! `[announcement]` tables of `site.toml`.

use std::fmt;

use serde::Deserialize;

use crate::{AbsoluteUrl, Locale};

/// The menus every page shows. Both are empty by default, and an empty menu
/// shows nothing:
///
/// ```toml
/// [[nav.primary]]
/// label = "Guides"
/// href = "/blog/guides/"
///
/// [[nav.footer]]
/// label = "Impressum"
/// href = "/blog/de/impressum/"
/// language = "de"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct NavSettings {
    /// The main menu in the header, in the order written.
    pub primary: Vec<MenuLink>,
    /// The menu in the footer, in the order written.
    pub footer: Vec<MenuLink>,
}

impl NavSettings {
    /// Every link of both menus that names a language.
    pub(crate) fn languages(&self) -> impl Iterator<Item = &Locale> {
        let links = self.primary.iter().chain(&self.footer);
        links.filter_map(|link| link.language.as_ref())
    }
}

/// One link in a menu.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct MenuLink {
    /// The words a reader sees.
    pub label: OneLine,
    /// Where it goes.
    pub href: SiteHref,
    /// The one language whose pages show it. `None` shows it on every page.
    #[serde(default)]
    pub language: Option<Locale>,
}

/// One line shown above the header of every page, such as news of a launch:
///
/// ```toml
/// [announcement]
/// text = "Our technical SEO course opens on 3 November."
/// href = "/blog/course/"
/// ```
///
/// It is plain text in the page, so it reads the same with scripts off.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Announcement {
    /// What it says.
    pub text: OneLine,
    /// Where the line links to. `None` shows it as text alone.
    #[serde(default)]
    pub href: Option<SiteHref>,
    /// The one language whose pages show it. `None` shows it on every page.
    #[serde(default)]
    pub language: Option<Locale>,
}

/// Text for one line of a page: not blank, no line breaks, and at most
/// [`OneLine::MAX_CHARS`] characters, kept as written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct OneLine(String);

impl OneLine {
    /// The most characters one line may hold, so it fits a phone screen in
    /// two or three rows.
    pub const MAX_CHARS: usize = 160;

    /// Checks `text`.
    ///
    /// # Errors
    ///
    /// Returns [`SiteNavError::Blank`], [`SiteNavError::LineBreak`] or
    /// [`SiteNavError::TooLong`].
    pub fn try_new(text: String) -> Result<Self, SiteNavError> {
        if text.trim().is_empty() {
            return Err(SiteNavError::Blank);
        }
        if text.contains(['\n', '\r']) {
            return Err(SiteNavError::LineBreak);
        }
        if text.chars().count() > Self::MAX_CHARS {
            return Err(SiteNavError::TooLong);
        }
        Ok(Self(text))
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OneLine {
    type Error = SiteNavError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::try_new(text)
    }
}

impl fmt::Display for OneLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a menu link or the announcement goes: a path on this site, such as
/// `/blog/about/`, written as the browser asks for it, or a full `http` or
/// `https` address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct SiteHref(String);

impl SiteHref {
    /// Checks `text`.
    ///
    /// # Errors
    ///
    /// Returns [`SiteNavError::Href`] for anything but a path that starts
    /// with one `/` and holds no spaces, or a full address that
    /// [`AbsoluteUrl::parse`] takes.
    pub fn try_new(text: String) -> Result<Self, SiteNavError> {
        let path = text.starts_with('/')
            && !text.starts_with("//")
            && !text.contains(|c: char| c.is_whitespace() || c.is_control() || c == '\\');
        if path || AbsoluteUrl::parse(&text).is_ok() {
            Ok(Self(text))
        } else {
            Err(SiteNavError::Href)
        }
    }

    /// The address as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SiteHref {
    type Error = SiteNavError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::try_new(text)
    }
}

impl fmt::Display for SiteHref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a menu or announcement value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SiteNavError {
    /// The text is empty or only spaces.
    #[error("write the words a reader sees here")]
    Blank,
    /// The text spans more than one line.
    #[error("keep this on one line")]
    LineBreak,
    /// The text is longer than [`OneLine::MAX_CHARS`].
    #[error("keep this to 160 characters or fewer")]
    TooLong,
    /// The address is neither a path on this site nor a full address.
    #[error(
        "use a path on this site such as /blog/about/, or a full address such as https://www.example.com/"
    )]
    Href,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn href(text: &str) -> Result<SiteHref, SiteNavError> {
        SiteHref::try_new(text.to_owned())
    }

    #[test]
    fn takes_a_path_on_the_site_or_a_full_address() {
        for good in [
            "/blog/about/",
            "/",
            "https://www.example.com/shop/",
            "http://example.com",
        ] {
            assert_eq!(href(good).map(|h| h.to_string()), Ok(good.to_owned()));
        }
    }

    #[test]
    fn refuses_anything_a_browser_could_read_another_way() {
        for bad in [
            "blog/about/",
            "//evil.example/",
            "/a b/",
            "/a\\b/",
            "javascript:alert(1)",
            "mailto:me@example.com",
            "",
        ] {
            assert_eq!(href(bad), Err(SiteNavError::Href), "{bad:?}");
        }
    }

    #[test]
    fn one_line_is_not_blank_has_no_break_and_fits() {
        let line = |text: &str| OneLine::try_new(text.to_owned());
        assert_eq!(line("  "), Err(SiteNavError::Blank));
        assert_eq!(line("one\ntwo"), Err(SiteNavError::LineBreak));
        assert_eq!(line(&"a".repeat(161)), Err(SiteNavError::TooLong));
        assert_eq!(line(&"ü".repeat(160)).map(|l| l.as_str().len()), Ok(320));
    }
}
