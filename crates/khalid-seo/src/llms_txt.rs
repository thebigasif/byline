//! `llms.txt`: a markdown guide to the blog for language models, in the
//! format proposed at llmstxt.org.

use std::fmt;

use khalid_core::AbsoluteUrl;

use crate::sitemap::is_listed;
use crate::{PageKind, SeoError, SeoPage, SeoSite, SitemapSection, UrlPath};

/// Where the file lives, inside the mount folder.
const LLMS_PATH: &str = "/llms.txt";

/// The address of `llms.txt`: in the mount folder.
///
/// # Errors
///
/// [`SeoError::Url`] when the address cannot be built.
pub fn llms_txt_url(site: &SeoSite) -> Result<AbsoluteUrl, SeoError> {
    site.url_for(&site.default_language, &UrlPath::try_new(LLMS_PATH)?)
}

/// One link line: `- [title](url): description`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Link {
    title: String,
    url: AbsoluteUrl,
    description: String,
}

/// One `##` section.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Section {
    heading: &'static str,
    links: Vec<Link>,
}

/// The blog's `llms.txt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmsTxt {
    title: String,
    summary: String,
    sections: Vec<Section>,
}

impl LlmsTxt {
    /// The `llms.txt` for `site`: the site name, its tagline as the summary,
    /// then every listed post and page with its excerpt, in the order given.
    /// Archives go under `Optional`, which the format marks as safe to skip.
    /// The home page is the file itself, so it is not listed.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when a page's address cannot be built.
    pub fn for_site(site: &SeoSite, pages: &[SeoPage]) -> Result<Self, SeoError> {
        let mut sections = vec![
            Section {
                heading: "Posts",
                links: Vec::new(),
            },
            Section {
                heading: "Pages",
                links: Vec::new(),
            },
            Section {
                heading: "Optional",
                links: Vec::new(),
            },
        ];
        for page in pages
            .iter()
            .filter(|page| page.kind != PageKind::Home && is_listed(site, page))
        {
            let slot = match SitemapSection::of(page.kind) {
                SitemapSection::Posts => 0,
                SitemapSection::Pages => 1,
                SitemapSection::Archives => 2,
            };
            if let Some(section) = sections.get_mut(slot) {
                section.links.push(Link {
                    title: page.title.clone(),
                    url: site.url_for(&page.language, &page.path)?,
                    description: page
                        .excerpt
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                });
            }
        }
        sections.retain(|section| !section.links.is_empty());
        Ok(Self {
            title: site.name.clone(),
            summary: site.tagline.clone(),
            sections,
        })
    }
}

/// Writes text inside a markdown link label, with its brackets escaped.
fn write_label(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    for c in text.chars() {
        if matches!(c, '[' | ']' | '\\') {
            f.write_str("\\")?;
        }
        write!(f, "{c}")?;
    }
    Ok(())
}

impl fmt::Display for LlmsTxt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "# {}", self.title)?;
        if !self.summary.is_empty() {
            writeln!(f, "\n> {}", self.summary)?;
        }
        for section in &self.sections {
            writeln!(f, "\n## {}\n", section.heading)?;
            for link in &section.links {
                f.write_str("- [")?;
                write_label(f, &link.title)?;
                write!(f, "]({})", link.url)?;
                if !link.description.is_empty() {
                    write!(f, ": {}", link.description)?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
