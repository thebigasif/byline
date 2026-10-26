//! The site's name, logo, share image and icons, from its home page.

use khalid_core::AbsoluteUrl;
use scraper::{ElementRef, Html, Selector};
use url::Url;

use crate::links::{base_address, has_rel};

/// What separates a site's name from the rest of a title, as in
/// `Acme | Dashboards for teams`.
const TITLE_SEPARATORS: [&str; 5] = [" | ", " – ", " - ", " · ", " : "];

/// The parts of a site's identity theme sync carries over to the blog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Brand {
    /// The site's name, from `og:site_name` or the first part of the title.
    pub name: Option<String>,
    /// The logo: an image marked as a logo, or else the first image in the
    /// page header.
    pub logo: Option<AbsoluteUrl>,
    /// The image other sites show when the home page is shared, from
    /// `og:image` or `twitter:image`.
    pub share_image: Option<AbsoluteUrl>,
    /// The icon links, such as the favicon and the Apple touch icon, in page
    /// order.
    pub icons: Vec<AbsoluteUrl>,
}

impl Brand {
    /// Reads the brand from the home page `html`, read from `home`.
    ///
    /// Addresses resolve against `<base href>` when there is one. Only
    /// `http` and `https` addresses are kept, so a `data:` or
    /// `javascript:` link never reaches the theme.
    pub fn from_html(html: &str, home: &AbsoluteUrl) -> Self {
        let Ok(home) = Url::parse(home.as_str()) else {
            return Self::default();
        };
        let document = Html::parse_document(html);
        let base = base_address(&document, &home);
        let page = Page {
            document: &document,
            base: &base,
        };
        Self {
            name: page
                .meta(&["og:site_name", "application-name"])
                .or_else(|| page.title()),
            logo: page.logo(),
            share_image: page
                .meta(&["og:image", "og:image:url", "twitter:image"])
                .and_then(|src| page.resolve(&src)),
            icons: page.icons(),
        }
    }
}

/// A parsed home page and the address its links resolve against.
struct Page<'a> {
    document: &'a Html,
    base: &'a Url,
}

impl Page<'_> {
    /// Every element `css` selects, in page order.
    fn select(&self, css: &str) -> Vec<ElementRef<'_>> {
        Selector::parse(css)
            .map(|selector| self.document.select(&selector).collect())
            .unwrap_or_default()
    }

    /// `href` as a full web address, or `None` for any other kind of link.
    fn resolve(&self, href: &str) -> Option<AbsoluteUrl> {
        let url = self.base.join(href.trim()).ok()?;
        AbsoluteUrl::parse(url.as_str()).ok()
    }

    /// The content of the first `<meta>` whose `property` or `name` is one
    /// of `names`, in the order `names` lists them.
    fn meta(&self, names: &[&str]) -> Option<String> {
        let metas = self.select("meta[content]");
        names.iter().find_map(|name| {
            metas.iter().find_map(|meta| {
                let element = meta.value();
                let key = element.attr("property").or_else(|| element.attr("name"))?;
                let content = element.attr("content")?.trim();
                (key.eq_ignore_ascii_case(name) && !content.is_empty()).then(|| content.to_owned())
            })
        })
    }

    /// The part of `<title>` before a separator such as ` | `.
    fn title(&self) -> Option<String> {
        let title: String = self.select("title").first()?.text().collect();
        let title = title.trim();
        let name = TITLE_SEPARATORS
            .iter()
            .filter_map(|separator| title.split_once(separator).map(|(name, _)| name))
            .min_by_key(|name| name.len())
            .unwrap_or(title)
            .trim();
        (!name.is_empty()).then(|| name.to_owned())
    }

    /// The first image marked as a logo, in the header first, or else the
    /// first image in the header.
    fn logo(&self) -> Option<AbsoluteUrl> {
        let header = self.select("header img[src]");
        let all = self.select("img[src]");
        let src = |image: &ElementRef<'_>| self.resolve(image.value().attr("src")?);
        header
            .iter()
            .chain(&all)
            .filter(|image| is_marked_logo(image))
            .find_map(src)
            .or_else(|| header.iter().find_map(src))
    }

    /// The icon links, in page order.
    fn icons(&self) -> Vec<AbsoluteUrl> {
        self.select("link[rel][href]")
            .iter()
            .filter(|link| {
                link.value()
                    .attr("rel")
                    .is_some_and(|rel| has_rel(rel, &["icon", "apple-touch-icon", "mask-icon"]))
            })
            .filter_map(|link| self.resolve(link.value().attr("href")?))
            .collect()
    }
}

/// Whether an image's address, alt text, class or id says it is a logo.
fn is_marked_logo(image: &ElementRef<'_>) -> bool {
    let element = image.value();
    ["src", "alt", "class", "id"]
        .iter()
        .filter_map(|name| element.attr(name))
        .any(|value| value.to_ascii_lowercase().contains("logo"))
}
