//! [`SeoSite`]: the site-wide settings every SEO output reads.

use khalid_core::{AbsoluteUrl, Locale};

use crate::{
    IndexPolicy, MetaTemplates, RootFile, SeoError, ShareImage, SiteVerification, TwitterHandle,
    UrlPath,
};

/// The site-wide settings every SEO output reads.
///
/// Build it with [`SeoSite::new`], then set the fields that differ from the
/// defaults. The wiring in task 1.15 fills it from `khalid_core::Config`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SeoSite {
    /// The site name, as readers see it: "Field Notes on SEO".
    pub name: String,
    /// One line about the site, the `{tagline}` variable.
    pub tagline: String,
    /// The separator the `{sep}` variable stands for. `|` by default.
    pub separator: String,
    /// Title and description templates per page type.
    pub templates: MetaTemplates,
    /// Which page types search engines may index.
    pub indexing: IndexPolicy,
    /// The public address of the site, such as `https://www.example.com`.
    pub address: AbsoluteUrl,
    /// The folder the blog is mounted under, such as `/blog/`. `/` for none.
    pub mount: UrlPath,
    /// The language whose pages have no language prefix.
    pub default_language: Locale,
    /// Every language the site publishes in, default first.
    pub languages: Vec<Locale>,
    /// The share image for pages that have none of their own.
    pub share_image: Option<ShareImage>,
    /// The site's Twitter or X account, the `twitter:site` tag.
    pub twitter_site: Option<TwitterHandle>,
    /// Verification codes, written as tags on the home page.
    pub verification: SiteVerification,
    /// Verification files served from the root of the address.
    pub root_files: Vec<RootFile>,
}

impl SeoSite {
    /// A site with one language, no mount folder, `|` as the separator and
    /// the default templates and index policy.
    pub fn new(name: &str, address: AbsoluteUrl, default_language: Locale) -> Self {
        Self {
            name: name.to_owned(),
            tagline: String::new(),
            separator: "|".to_owned(),
            templates: MetaTemplates::default(),
            indexing: IndexPolicy::default(),
            address,
            mount: UrlPath::root(),
            languages: vec![default_language.clone()],
            default_language,
            share_image: None,
            twitter_site: None,
            verification: SiteVerification::default(),
            root_files: Vec::new(),
        }
    }

    /// The absolute URL of `path` in `language`: the address, then the mount
    /// folder, then the language prefix for any language but the default,
    /// then the path. A query or fragment on the address is dropped.
    ///
    /// `/why-links-matter/` in German on the demo site becomes
    /// `https://www.example.com/blog/de/why-links-matter/`.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when the joined address is not a valid URL.
    pub fn url_for(&self, language: &Locale, path: &UrlPath) -> Result<AbsoluteUrl, SeoError> {
        let address = self.address.as_str();
        let without_query = address.split(['?', '#']).next().unwrap_or(address);
        let mut full = without_query.trim_end_matches('/').to_owned();
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
    fn a_query_or_fragment_on_the_address_is_dropped() {
        let site = site("https://example.com/?ref=home#top", "/blog/");
        assert_eq!(
            url(&site, "en", "/links/"),
            "https://example.com/blog/links/"
        );
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
