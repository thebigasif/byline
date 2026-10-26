//! [`SeoSite`]: the site-wide settings every SEO output reads.

use khalid_core::{AbsoluteUrl, Locale};

use crate::{SeoError, UrlPath};

/// The site-wide settings every SEO output reads.
///
/// Build it with [`SeoSite::new`], then set the fields that differ from the
/// defaults. The wiring in task 1.15 fills it from `khalid_core::Config`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SeoSite {
    /// The site name, as readers see it: "Field Notes on SEO".
    pub name: String,
    /// The public address of the site, such as `https://www.example.com`.
    pub address: AbsoluteUrl,
    /// The folder the blog is mounted under, such as `/blog/`. `/` for none.
    pub mount: UrlPath,
    /// The language whose pages have no language prefix.
    pub default_language: Locale,
    /// Every language the site publishes in, default first.
    pub languages: Vec<Locale>,
}

impl SeoSite {
    /// A site with one language and no mount folder.
    pub fn new(name: &str, address: AbsoluteUrl, default_language: Locale) -> Self {
        Self {
            name: name.to_owned(),
            address,
            mount: UrlPath::root(),
            languages: vec![default_language.clone()],
            default_language,
        }
    }

    /// The absolute URL of `path` in `language`: the address, then the mount
    /// folder, then the language prefix for any language but the default,
    /// then the path.
    ///
    /// `/why-links-matter/` in German on the demo site becomes
    /// `https://www.example.com/blog/de/why-links-matter/`.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when the joined address is not a valid URL.
    pub fn url_for(&self, language: &Locale, path: &UrlPath) -> Result<AbsoluteUrl, SeoError> {
        let mut full = self.address.as_str().trim_end_matches('/').to_owned();
        full.push_str(self.mount.without_trailing_slash());
        if language != &self.default_language {
            full.push('/');
            full.push_str(&language.as_str().to_ascii_lowercase());
        }
        full.push_str(path.as_str());
        AbsoluteUrl::parse(&full).map_err(|source| SeoError::Url {
            path: path.as_str().to_owned(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use khalid_core::{AbsoluteUrl, Locale};

    use super::SeoSite;
    use crate::UrlPath;

    fn site(address: &str, mount: &str) -> SeoSite {
        let mut site = SeoSite::new(
            "Field Notes on SEO",
            AbsoluteUrl::parse(address).unwrap(),
            Locale::try_new("en").unwrap(),
        );
        site.mount = UrlPath::try_new(mount).unwrap();
        site
    }

    fn url(site: &SeoSite, language: &str, path: &str) -> String {
        let language = Locale::try_new(language).unwrap();
        let path = UrlPath::try_new(path).unwrap();
        site.url_for(&language, &path).unwrap().to_string()
    }

    #[test]
    fn joins_address_mount_language_and_path_with_single_slashes() {
        let site = site("https://www.example.com/", "/blog/");
        assert_eq!(url(&site, "en", "/"), "https://www.example.com/blog/");
        assert_eq!(url(&site, "de", "/"), "https://www.example.com/blog/de/");
        assert_eq!(
            url(&site, "pt-BR", "/links/"),
            "https://www.example.com/blog/pt-br/links/"
        );
    }

    #[test]
    fn a_site_at_the_root_has_no_mount_folder() {
        let site = site("https://example.com", "/");
        assert_eq!(url(&site, "en", "/links/"), "https://example.com/links/");
    }

    #[test]
    fn letters_outside_ascii_are_percent_encoded() {
        let site = site("https://example.com", "/");
        assert_eq!(
            url(&site, "de", "/über-uns/"),
            "https://example.com/de/%C3%BCber-uns/"
        );
    }
}
