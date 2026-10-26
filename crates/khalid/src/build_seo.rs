//! The SEO side of `khalid build`: the site and each page as `khalid-seo`
//! reads them (ADR 0013), and the head every page carries: its tags, then
//! its JSON-LD graph (ADR "the SEO head reaches templates as trusted HTML").

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use khalid_core::{Config, Locale, UserMessage};
use khalid_render::HeadHtml;
use khalid_seo::{
    PageKind, SeoError, SeoPage, SeoSite, TranslationKey, UrlPath, head_tags, link_translations,
    page_graph,
};

use crate::build_message;
use crate::build_pages::LanguagePages;

/// The site as the SEO outputs read it, from `site.toml`: its name, address,
/// mount folder and languages, default first.
pub(crate) fn site(config: &Config) -> Result<SeoSite, UserMessage> {
    let mut site = SeoSite::new(
        config.name(),
        config.address().clone(),
        config.default_language().clone(),
    );
    site.mount = UrlPath::try_new(config.mount()).map_err(|error| build_message::seo(&error))?;
    let others = config
        .languages()
        .iter()
        .filter(|locale| *locale != config.default_language());
    site.languages.extend(others.cloned());
    Ok(site)
}

/// A page of `kind` in `locale` whose path in the mount folder and language
/// prefix ends in `rest`, such as `why-internal-links-matter/`.
pub(crate) fn page(
    kind: PageKind,
    locale: &Locale,
    rest: &str,
    title: &str,
) -> Result<SeoPage, UserMessage> {
    let path = UrlPath::try_new(&format!("/{rest}")).map_err(|error| build_message::seo(&error))?;
    Ok(SeoPage::new(kind, locale.clone(), path, title))
}

/// The `translation` key from front matter, which pairs language versions.
pub(crate) fn translation(key: Option<&str>) -> Result<Option<TranslationKey>, UserMessage> {
    key.map(TranslationKey::try_new)
        .transpose()
        .map_err(|error| build_message::seo(&error))
}

/// The start of `date` in UTC, for a post dated by day.
pub(crate) fn midnight(date: NaiveDate) -> DateTime<Utc> {
    date.and_time(NaiveTime::MIN).and_utc()
}

/// Links the language versions of every page, then gives each page its
/// head. Returns the pages with their versions linked, in the order of
/// `languages`, for the site's files.
pub(crate) fn heads(
    site: &SeoSite,
    languages: &mut [LanguagePages],
) -> Result<Vec<SeoPage>, UserMessage> {
    let mut seo: Vec<SeoPage> = languages
        .iter()
        .flat_map(|language| &language.pages)
        .map(|page| page.seo.clone())
        .collect();
    link_translations(site, &mut seo).map_err(|error| build_message::seo(&error))?;
    let pages = languages
        .iter_mut()
        .flat_map(|language| &mut language.pages);
    for (page, linked) in pages.zip(&seo) {
        let head = head(site, linked).map_err(|error| build_message::seo(&error))?;
        page.meta.seo_head = Some(head);
    }
    Ok(seo)
}

/// The SEO tags of `page`, then its schema graph in one script tag.
fn head(site: &SeoSite, page: &SeoPage) -> Result<HeadHtml, SeoError> {
    let mut html = head_tags(site, page)?.to_html();
    html.push_str(&page_graph(site, page)?.to_script_tag());
    Ok(HeadHtml::from_trusted(html))
}
