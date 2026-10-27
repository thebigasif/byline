//! The SEO side of `khalid build`: the site and each page as `khalid-seo`
//! reads them (ADR 0013), and the head every page carries: its tags, then
//! its JSON-LD graph (ADR "the SEO head reaches templates as trusted HTML").

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use khalid_content::{PostFront, Sitemap};
use khalid_core::{Config, Locale, UserMessage};
use khalid_lint::StructuredDataGap;
use khalid_render::HeadHtml;
use khalid_seo::{
    IndexPolicy, IssueLevel, LinkPolicy, MetaTemplate, PageKind, SeoError, SeoPage, SeoSite,
    SitemapListing, TranslationKey, UrlPath, check_graph, head_tags, link_translations, page_graph,
};

use crate::build_pages::LanguagePages;
use crate::{build_message, search_page};

/// The site as the SEO outputs read it, from `site.toml`: its name, address,
/// mount folder, languages, default first, and which archives are indexed.
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
    site.indexing = IndexPolicy::from(&config.seo().index);
    site.links = LinkPolicy::from(&config.seo().links);
    search_page::announce(config, &mut site)?;
    Ok(site)
}

/// `html`, a rendered body, with each link given the `rel` and `target` the
/// link policy in `[seo.links]` gives it. Without that table, `html` as it
/// is.
pub(crate) fn with_link_policy(config: &Config, html: String) -> String {
    let policy = LinkPolicy::from(&config.seo().links);
    if policy == LinkPolicy::default() {
        return html;
    }
    let site = SeoSite::new(
        config.name(),
        config.address().clone(),
        config.default_language().clone(),
    );
    policy.apply_to_html(&site, &html)
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

/// The SEO page of the post with front matter `front`, dated `date`, whose
/// path ends in `rest` and whose title is `title`: the settings its head
/// and schema graph are made from, before its body, author and reviewers
/// are added. `khalid build` and the linter both start from it, so the
/// linter checks the graph the page carries.
pub(crate) fn post(
    locale: &Locale,
    (rest, title): (&str, &str),
    front: &PostFront,
    date: NaiveDate,
) -> Result<SeoPage, UserMessage> {
    let mut seo = page(PageKind::Post, locale, rest, title)?;
    seo.excerpt = front.description.clone().unwrap_or_default();
    seo.published = Some(midnight(date));
    from_front(&mut seo, front)?;
    Ok(seo)
}

/// The properties Google needs that the schema graph of `page` on `site`
/// lacks, as the linter reads them. Properties Google only recommends are
/// left out: they never stop a post.
pub(crate) fn structured_data_gaps(site: &SeoSite, page: &SeoPage) -> Vec<StructuredDataGap> {
    let Ok(graph) = page_graph(site, page) else {
        return Vec::new();
    };
    check_graph(&graph)
        .into_iter()
        .filter(|issue| issue.level() == IssueLevel::Blocks)
        .map(|issue| StructuredDataGap::new(issue.kind(), issue.property()))
        .collect()
}

/// `html` with each `href` and `src` that starts at the site root, such as
/// `/blog/about/`, made a full address on the site's host, for feed readers,
/// which have no page to read such a path against. A body escapes `"` in its
/// text, so `="/` can only end an attribute name.
pub(crate) fn full_addresses(config: &Config, html: &str) -> String {
    let origin = config.address().as_str().trim_end_matches('/');
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once("=\"/") {
        out.push_str(before);
        out.push_str("=\"");
        let named = before.ends_with(" href") || before.ends_with(" src");
        if named && !after.starts_with('/') {
            out.push_str(origin);
        }
        out.push('/');
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Copies what a post's `front` matter says about search into `seo`: its
/// translation key, robots, canonical, search title and sitemap listing.
/// The search title may use the meta template variables, such as `{site}`.
pub(crate) fn from_front(seo: &mut SeoPage, front: &PostFront) -> Result<(), UserMessage> {
    seo.translation = translation(front.translation.as_deref())?;
    seo.robots.clone_from(&front.robots);
    seo.canonical.clone_from(&front.canonical);
    let title = front.seo_title.as_deref().map(MetaTemplate::parse);
    seo.seo_title = title
        .transpose()
        .map_err(|error| build_message::seo(&error))?;
    seo.sitemap = match front.sitemap {
        Sitemap::Unlisted => SitemapListing::Excluded,
        _ => SitemapListing::Listed,
    };
    Ok(())
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use khalid_content::parse_post;
    use khalid_core::Environment;
    use khalid_seo::CustomSchema;

    use super::*;

    /// The demo site, and a post page on it, as `khalid build` starts it.
    fn demo_post() -> (SeoSite, SeoPage) {
        let mut folder = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        folder.push("../../fixtures/demo-site");
        let config = Config::load(&folder, &Environment::empty()).unwrap();
        let text = "---\ntitle: Why links matter\nauthor: asif\n---\n\nA post.\n";
        let post = khalid_content::parse_post(Path::new("why.md"), text).unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        let locale = config.default_language().clone();
        let mut page = super::post(
            &locale,
            ("why-links-matter/", "Why links matter"),
            &post.front,
            date,
        )
        .unwrap();
        page.author = Some("asif".to_owned());
        (site(&config).unwrap(), page)
    }

    #[test]
    fn a_post_page_as_built_today_lacks_nothing_google_needs() {
        let (site, page) = demo_post();
        assert_eq!(structured_data_gaps(&site, &page), []);
    }

    #[test]
    fn each_property_google_needs_and_the_graph_lacks_is_a_gap() {
        let (site, mut page) = demo_post();
        let event = r#"{"@context": "https://schema.org", "@type": "Event", "name": "Link clinic", "location": "Lisbon"}"#;
        page.custom_schema = CustomSchema::parse(event).unwrap();
        assert_eq!(
            structured_data_gaps(&site, &page),
            [StructuredDataGap::new("Event", "startDate")]
        );
    }

    #[test]
    fn a_posts_search_title_and_sitemap_choice_reach_its_seo_page() {
        let text = "---\nseo_title: \"Slow pages, fixed {sep} {site}\"\nsitemap: false\n---\n";
        let front = parse_post(Path::new("a.md"), text).unwrap().front;
        let locale = Locale::try_new("en").unwrap();
        let mut seo = page(PageKind::Post, &locale, "slow/", "Slow pages").unwrap();
        from_front(&mut seo, &front).unwrap();
        let title = seo.seo_title.as_ref().map(MetaTemplate::as_str);
        assert_eq!(title, Some("Slow pages, fixed {sep} {site}"));
        assert_eq!(seo.sitemap, SitemapListing::Excluded);
        let plain = parse_post(Path::new("a.md"), "---\n---\n").unwrap().front;
        from_front(&mut seo, &plain).unwrap();
        assert_eq!((seo.seo_title, seo.sitemap), (None, SitemapListing::Listed));
    }
}
