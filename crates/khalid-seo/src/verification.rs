//! Site verification: the meta tags and root files search engines check
//! to confirm who owns a site.

use crate::{HeadTag, SeoError, UrlPath};

/// Root file names the site writes itself, which a verification file may not take.
const RESERVED_ROOT_FILES: [&str; 2] = ["robots.txt", "llms.txt"];

/// A verification code a search engine gave the site owner.
///
/// It is public: it appears in every copy of the home page. It holds ASCII
/// letters, digits and `-`, `_`, `.`, `=`, `+` and `/` only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VerificationCode(String);

impl VerificationCode {
    /// Checks a code and wraps it.
    ///
    /// # Errors
    ///
    /// [`SeoError::VerificationCode`] when the code is empty or holds any
    /// other character, such as a space, a quote or a bracket.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        let valid = !text.is_empty()
            && text.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '=' | '+' | '/')
            });
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(SeoError::VerificationCode {
                code: text.to_owned(),
            })
        }
    }

    /// The code as given.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The verification codes a site has, one per search engine.
///
/// Each one becomes a `<meta>` tag on the blog's home page, which is the
/// page every engine checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SiteVerification {
    /// Google Search Console: `google-site-verification`.
    pub google: Option<VerificationCode>,
    /// Bing Webmaster Tools: `msvalidate.01`.
    pub bing: Option<VerificationCode>,
    /// Yandex Webmaster: `yandex-verification`.
    pub yandex: Option<VerificationCode>,
    /// Baidu Search Resource Platform: `baidu-site-verification`.
    pub baidu: Option<VerificationCode>,
    /// Pinterest: `p:domain_verify`.
    pub pinterest: Option<VerificationCode>,
}

impl SiteVerification {
    /// Appends one `<meta>` tag per code to `out`, in a fixed order.
    pub(crate) fn push_tags(&self, out: &mut Vec<HeadTag>) {
        let tags = [
            ("google-site-verification", &self.google),
            ("msvalidate.01", &self.bing),
            ("yandex-verification", &self.yandex),
            ("baidu-site-verification", &self.baidu),
            ("p:domain_verify", &self.pinterest),
        ];
        for (name, code) in tags {
            if let Some(code) = code {
                out.push(HeadTag::Meta {
                    name,
                    content: code.as_str().to_owned(),
                });
            }
        }
    }
}

/// A verification file served from the root of the site's address, such as
/// `/google1a2b3c4d5e6f7a8b.html`.
///
/// The root is the address's root, not the mount folder, because that is
/// where search engines look. When the blog is mounted under another site,
/// that site must pass the file through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootFile {
    path: UrlPath,
    body: String,
}

impl RootFile {
    /// Checks a file name and pairs it with the text the file holds.
    ///
    /// # Errors
    ///
    /// [`SeoError::RootFile`] when the name is empty, starts with a dot, holds
    /// anything but ASCII letters, digits, `-`, `_` and `.`, or is a file the
    /// site writes itself: `robots.txt`, `llms.txt` or a name starting with `sitemap`.
    pub fn try_new(name: &str, body: &str) -> Result<Self, SeoError> {
        let refused = |problem| SeoError::RootFile {
            name: name.to_owned(),
            problem,
        };
        if name.is_empty() || name.starts_with('.') {
            return Err(refused("is empty or starts with a dot"));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(refused(
                "holds a character other than letters, digits, -, _ and .",
            ));
        }
        let lower = name.to_ascii_lowercase();
        if RESERVED_ROOT_FILES.contains(&lower.as_str()) || lower.starts_with("sitemap") {
            return Err(refused("is a file the site writes itself"));
        }
        let path = UrlPath::try_new(&format!("/{name}"))?;
        Ok(Self {
            path,
            body: body.to_owned(),
        })
    }

    /// Where the file is served, such as `/google1a2b3c4d5e6f7a8b.html`.
    pub fn path(&self) -> &UrlPath {
        &self.path
    }

    /// The text the file holds.
    pub fn body(&self) -> &str {
        &self.body
    }
}
