//! The files a crawler reads before the pages: `robots.txt`, the sitemaps
//! it names, and `llms.txt`.

use khalid_core::AbsoluteUrl;
use url::Url;

use crate::{Answer, Source};

/// Sitemap files are followed this many levels deep, an index and the
/// sitemaps it lists, and no further.
const MOST_SITEMAP_LEVELS: usize = 2;

/// One sitemap file the crawl read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sitemap {
    url: AbsoluteUrl,
    status: u16,
    pages: Vec<AbsoluteUrl>,
}

impl Sitemap {
    /// Where the sitemap is.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// The status it answered with, or 0 when nothing answered.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The pages it lists, in order. An index lists none itself.
    pub fn pages(&self) -> &[AbsoluteUrl] {
        &self.pages
    }
}

/// The files a crawler reads before the pages.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteFiles {
    robots: Option<String>,
    llms: Option<String>,
    sitemaps: Vec<Sitemap>,
    named: bool,
}

impl SiteFiles {
    /// Whether `robots.txt` named the sitemaps, rather than the crawl
    /// trying the blog's `sitemap.xml`.
    pub fn named_in_robots(&self) -> bool {
        self.named
    }

    /// The text of `robots.txt`, when it answered 200.
    pub fn robots(&self) -> Option<&str> {
        self.robots.as_deref()
    }

    /// The text of `llms.txt`, when it answered 200.
    pub fn llms(&self) -> Option<&str> {
        self.llms.as_deref()
    }

    /// Every sitemap read, indexes first.
    pub fn sitemaps(&self) -> &[Sitemap] {
        &self.sitemaps
    }

    /// Every page the sitemaps list, in order.
    pub fn listed(&self) -> impl Iterator<Item = &AbsoluteUrl> {
        self.sitemaps
            .iter()
            .flat_map(|sitemap| sitemap.pages.iter())
    }

    /// Reads `robots.txt` at the root of the host, where search engines
    /// look for it even for a blog in a folder, then the blog's `llms.txt`
    /// and its sitemaps from `source`. The sitemaps are those `robots.txt`
    /// names inside the blog, or the blog's `sitemap.xml` when it names
    /// none.
    pub(crate) async fn read<S: Source>(source: &S, home: &AbsoluteUrl) -> Self {
        let Ok(base) = Url::parse(home.as_str()) else {
            return Self::default();
        };
        let at = |name: &str| {
            base.join(name)
                .ok()
                .and_then(|url| AbsoluteUrl::parse(url.as_str()).ok())
        };
        let text = |answer: Answer| {
            (answer.status() == 200).then(|| String::from_utf8_lossy(answer.body()).into_owned())
        };
        let mut files = Self::default();
        if let Some(url) = at("/robots.txt") {
            files.robots = source.fetch(&url).await.ok().and_then(text);
        }
        if let Some(url) = at("llms.txt") {
            files.llms = source.fetch(&url).await.ok().and_then(text);
        }
        let named: Vec<AbsoluteUrl> = files
            .robots
            .as_deref()
            .map(sitemap_lines)
            .unwrap_or_default()
            .into_iter()
            .filter(|url| url.as_str().starts_with(home.as_str()))
            .collect();
        files.named = !named.is_empty();
        let mut queue = if named.is_empty() {
            at("sitemap.xml").into_iter().collect()
        } else {
            named
        };
        for _ in 0..MOST_SITEMAP_LEVELS {
            let mut next = Vec::new();
            for url in queue {
                if files.sitemaps.iter().any(|read| read.url == url) {
                    continue;
                }
                let answer = source.fetch(&url).await.ok();
                let status = answer.as_ref().map_or(0, Answer::status);
                let (pages, children) = answer
                    .filter(|answer| answer.status() == 200)
                    .map(|answer| locations(answer.body()))
                    .unwrap_or_default();
                next.extend(children);
                files.sitemaps.push(Sitemap { url, status, pages });
            }
            queue = next;
        }
        files
    }
}

/// The addresses on the `Sitemap:` lines of `robots`.
fn sitemap_lines(robots: &str) -> Vec<AbsoluteUrl> {
    robots
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("sitemap")
                .then(|| AbsoluteUrl::parse(value.trim()).ok())?
        })
        .collect()
}

/// The page addresses of a `<urlset>` and the sitemap addresses of a
/// `<sitemapindex>` in `xml`. XML that does not parse lists nothing.
fn locations(xml: &[u8]) -> (Vec<AbsoluteUrl>, Vec<AbsoluteUrl>) {
    let text = String::from_utf8_lossy(xml);
    let Ok(document) = roxmltree::Document::parse(&text) else {
        return (Vec::new(), Vec::new());
    };
    let mut pages = Vec::new();
    let mut sitemaps = Vec::new();
    for node in document
        .descendants()
        .filter(|node| node.has_tag_name("loc"))
    {
        let Some(url) = node
            .text()
            .and_then(|text| AbsoluteUrl::parse(text.trim()).ok())
        else {
            continue;
        };
        match node.parent().map(|parent| parent.tag_name().name()) {
            Some("sitemap") => sitemaps.push(url),
            Some("url") => pages.push(url),
            _ => {}
        }
    }
    (pages, sitemaps)
}
