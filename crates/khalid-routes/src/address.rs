//! A post's address, built from its collection's pattern, and the redirects
//! its aliases make.

use chrono::{Datelike, NaiveDate};
use khalid_core::{Config, Locale, Slug};

use crate::Routes;

/// The pattern of a collection `routes.toml` does not name.
const UNNAMED: &str = "/{collection}/{slug}/";

/// What a post's address is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostAddress<'a> {
    collection: &'a str,
    locale: &'a Locale,
    slug: &'a Slug,
    date: Option<NaiveDate>,
}

impl<'a> PostAddress<'a> {
    /// A post in `collection`, written in `locale`, with its slug and date.
    pub fn new(
        collection: &'a str,
        locale: &'a Locale,
        slug: &'a Slug,
        date: Option<NaiveDate>,
    ) -> Self {
        Self {
            collection,
            locale,
            slug,
            date,
        }
    }
}

/// An old address that sends readers, and search engines, to a post's
/// current one with a permanent redirect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    from: String,
    to: String,
}

impl Redirect {
    /// The old address, as written in the post's `aliases`.
    pub fn from(&self) -> &str {
        &self.from
    }

    /// The post's address now.
    pub fn to(&self) -> &str {
        &self.to
    }
}

/// Why an address or a redirect could not be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AddressError {
    /// The collection's pattern uses the date, and the post has none.
    #[error("the {collection} collection's permalink uses the date, and this post has none")]
    NoDate {
        /// The collection.
        collection: String,
    },
    /// An alias is not a clean path that starts with `/`.
    #[error("the alias {alias:?} is not a path such as /blog/old-post/")]
    NotAPath {
        /// The alias as written.
        alias: String,
    },
    /// An alias lies outside the mount path, where Khalid never sees a request.
    #[error("the alias {alias:?} is outside {mount}, where Khalid does not answer")]
    OutsideMount {
        /// The alias as written.
        alias: String,
        /// The site's mount path.
        mount: String,
    },
    /// An alias is the post's own address, which would redirect to itself.
    #[error("the alias {alias:?} is the post's own address, so it would redirect to itself")]
    SameAddress {
        /// The alias as written.
        alias: String,
    },
}

impl Routes {
    /// The post's address from the site root: the mount path, then the
    /// language prefix for every language but the default one, then the
    /// collection's pattern with its placeholders filled in. Months and days
    /// have two digits. Slugs are written as they are; percent-encode them
    /// where a protocol needs plain ASCII.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError::NoDate`] when the pattern uses the date and
    /// the post has none.
    pub fn address(&self, site: &Config, post: &PostAddress<'_>) -> Result<String, AddressError> {
        let pattern = self
            .collection(post.collection)
            .map_or(UNNAMED, |collection| collection.permalink().as_str());
        let mut filled = pattern.replace("{slug}", post.slug.as_str());
        if ["{year}", "{month}", "{day}"]
            .iter()
            .any(|part| filled.contains(part))
        {
            let date = post.date.ok_or_else(|| AddressError::NoDate {
                collection: post.collection.to_owned(),
            })?;
            filled = filled
                .replace("{year}", &format!("{:04}", date.year()))
                .replace("{month}", &format!("{:02}", date.month()))
                .replace("{day}", &format!("{:02}", date.day()));
        }
        // Last, so a collection name never has its own text replaced.
        let filled = filled.replace("{collection}", post.collection);
        let language = if post.locale == site.default_language() {
            String::new()
        } else {
            format!("{}/", post.locale)
        };
        let rest = filled.strip_prefix('/').unwrap_or(&filled);
        Ok(format!("{}{language}{rest}", site.mount()))
    }

    /// The redirect an alias in a post's front matter makes: from the old
    /// address to the post's address now.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError::NotAPath`] when the alias is not a clean path,
    /// [`AddressError::OutsideMount`] when it lies outside the mount path,
    /// [`AddressError::SameAddress`] when it is the post's own address, and
    /// the errors of [`Routes::address`].
    pub fn redirect(
        &self,
        site: &Config,
        alias: &str,
        post: &PostAddress<'_>,
    ) -> Result<Redirect, AddressError> {
        let clean = alias.starts_with('/')
            && !alias.contains("//")
            && !alias.split('/').any(|part| part == "." || part == "..")
            && !alias
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || c == '?' || c == '#');
        if !clean {
            return Err(AddressError::NotAPath {
                alias: alias.to_owned(),
            });
        }
        if !alias.starts_with(site.mount()) {
            return Err(AddressError::OutsideMount {
                alias: alias.to_owned(),
                mount: site.mount().to_owned(),
            });
        }
        let to = self.address(site, post)?;
        if to == alias {
            return Err(AddressError::SameAddress {
                alias: alias.to_owned(),
            });
        }
        Ok(Redirect {
            from: alias.to_owned(),
            to,
        })
    }
}
