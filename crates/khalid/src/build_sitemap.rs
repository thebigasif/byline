//! The HTML sitemap page of each language (plan section 5.2, audit brief
//! G12): every page the XML sitemap lists, grouped as posts, pages, and
//! topics and archives, at `/sitemap/`. It is made in two steps, because
//! which pages a sitemap lists is known only once every page's head is.

use khalid_core::{Config, UserMessage};
use khalid_render::{
    Link, PageBody, PageKind, RenderSitemap, Renderer, SitemapGroup, SitemapSection,
};
use khalid_seo::{
    HtmlSitemap, PageKind as SeoKind, SeoPage, SeoSite, SitemapSection as SeoSection,
};

use crate::build_pages::{self, LanguagePages, Page};
use crate::{build_lists, build_message, build_seo};

/// The end of the sitemap page's address, after the mount and the language.
const REST: &str = "sitemap/";

/// Adds the sitemap page of each of `languages`, titled in its language by
/// `renderer`'s strings and still empty: [`fill`] lists the pages.
///
/// # Errors
///
/// Returns what, why and fix when the page's address cannot be made from
/// the site's settings.
pub(crate) fn add(
    config: &Config,
    renderer: &Renderer,
    languages: &mut [LanguagePages],
) -> Result<(), UserMessage> {
    for language in languages.iter_mut() {
        let locale = language.site.locale.clone();
        let title = build_lists::text(renderer, &locale, "sitemap_title")?;
        let path = build_pages::path_of(config, &locale, REST);
        let meta = build_pages::meta(
            config,
            title.clone(),
            String::new(),
            path,
            PageKind::Website,
        )?;
        let mut seo = build_seo::page(SeoKind::Page, &locale, REST, &title)?;
        seo.translation = build_lists::list_key("sitemap")?;
        language.pages.push(Page {
            file: format!("{}{REST}index.html", build_pages::prefix(config, &locale)),
            meta,
            body: PageBody::Sitemap(RenderSitemap {
                title,
                groups: Vec::new(),
            }),
            seo,
            source: None,
            template: None,
            head_code: None,
            lead: None,
        });
    }
    Ok(())
}

/// Fills the sitemap page of each of `languages`, if it has one, with the
/// pages of `seo`, as `build_seo::heads` returns them, that `site`'s XML
/// sitemap lists in that language, the sitemap page itself left out.
///
/// # Errors
///
/// Returns what, why and fix when a page's address cannot be made.
pub(crate) fn fill(
    config: &Config,
    site: &SeoSite,
    seo: &[SeoPage],
    languages: &mut [LanguagePages],
) -> Result<(), UserMessage> {
    let origin = config.address().as_str().trim_end_matches('/');
    for language in languages.iter_mut() {
        let locale = &language.site.locale;
        let pages = language.pages.iter_mut();
        let Some(page) = pages
            .into_iter()
            .find(|page| matches!(page.body, PageBody::Sitemap(_)))
        else {
            continue;
        };
        let listed = HtmlSitemap::from_pages(site, seo, locale)
            .map_err(|error| build_message::seo(&error))?;
        let own = page.meta.path.clone();
        let groups = listed.groups.into_iter().map(|group| {
            let links = group.links.into_iter().map(|link| {
                let url = link.url.as_str();
                Link {
                    href: url.strip_prefix(origin).unwrap_or(url).to_owned(),
                    name: link.title,
                }
            });
            SitemapGroup {
                section: section(group.section),
                links: links.filter(|link| link.href != own).collect(),
            }
        });
        let groups = groups.filter(|group| !group.links.is_empty()).collect();
        if let PageBody::Sitemap(sitemap) = &mut page.body {
            sitemap.groups = groups;
        }
    }
    Ok(())
}

/// The render crate's name for a section of the sitemap.
fn section(section: SeoSection) -> SitemapSection {
    match section {
        SeoSection::Posts => SitemapSection::Posts,
        SeoSection::Pages => SitemapSection::Pages,
        _ => SitemapSection::Archives,
    }
}
