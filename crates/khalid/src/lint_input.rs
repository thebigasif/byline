//! Turns the content files into the linter's own model (ADR 0019).
//!
//! The lint crate never reads files, so this binary maps parsed posts and
//! their rendered output to `LintPost` and `LintSite`, as the built page has
//! them: the slug and address `khalid build` gives each post, its body
//! rendered with its wikilinks linked, the wikilinks as written, its forms,
//! focus keywords, collection and topic cluster, the `rel` words the link
//! policy gives each link, the site's collections, and the links of each
//! home page, which lists every dated post (audit 2, G5), the site's fact
//! registry (P7) and each post's brief (P6).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::NaiveDate;
use khalid_content::{
    BlockKind, Document, Link, PostFront, SiteContent, StructuredBlock, TopicCluster, TopicRole,
    Wikilinks, parse_blocks, render_linked,
};
use khalid_core::{Config, Locale, Slug};
use khalid_lint::{
    Cluster, ClusterRole, GlossaryTerm, Heading, HeadingLevel, Image, LinkKind, LintCollection,
    LintFact, LintForm, LintLink, LintPost, LintSite, PageRef, StructuredDataGap,
};
use khalid_routes::{PostAddress, Routes};
use khalid_seo::{LinkPolicy, SeoPage, SeoSite};

use crate::build_pages::{path_of, routed};
use crate::lint_text::{plain_text, section_shapes, wikilinks};
use crate::site_briefs::Briefs;
use crate::{build_schema, build_seo};

/// What a post is linted against besides itself: the site's routes, the
/// pages a wikilink reaches in each language, the site as the SEO outputs
/// read it, or `None` when `site.toml` cannot give one, in which case
/// `khalid build` stops with its own message, the site's fact registry and
/// the posts' briefs.
pub(crate) struct LintInput {
    routes: Routes,
    links: BTreeMap<Locale, Wikilinks>,
    seo: Option<SeoSite>,
    policy: (LinkPolicy, SeoSite),
    facts: Vec<LintFact>,
    briefs: Briefs,
}

impl LintInput {
    /// The input for the posts of `content` on the site `config` describes,
    /// whose addresses follow `routes`.
    pub(crate) fn new(content: &SiteContent, config: &Config, routes: Routes) -> Self {
        let links = config
            .languages()
            .iter()
            .map(|locale| {
                let links = Wikilinks::for_language(content, config, &routes, locale);
                (locale.clone(), links)
            })
            .collect();
        let seo = build_seo::site(config).ok();
        let policy = build_seo::link_policy(config);
        Self {
            routes,
            links,
            seo,
            policy,
            facts: Vec::new(),
            briefs: Briefs::new(),
        }
    }

    /// This input with the site's fact registry, read from `facts.toml`.
    pub(crate) fn with_facts(self, facts: Vec<LintFact>) -> Self {
        Self { facts, ..self }
    }

    /// This input with each post's brief, read from `briefs/`.
    pub(crate) fn with_briefs(self, briefs: Briefs) -> Self {
        Self { briefs, ..self }
    }
}

/// The site's posts, pages and glossary in the lint model, and the document
/// each post came from, in the same order.
pub(crate) fn lint_site<'c>(
    content: &'c SiteContent,
    config: &Config,
    input: &LintInput,
) -> (LintSite, Vec<&'c Document<PostFront>>) {
    let mut site = LintSite::default();
    let mut documents = Vec::new();
    for document in &content.posts {
        if let Some(post) = lint_post(document, config, input) {
            site.posts.push(post);
            documents.push(document);
        }
    }
    // Each home page lists every dated post of its language.
    site.home_links = site
        .posts
        .iter()
        .filter(|post| post.date.is_some())
        .filter_map(|post| post.path.clone())
        .collect();
    let used: BTreeSet<&str> = content
        .posts
        .iter()
        .map(|d| d.collection.as_str())
        .collect();
    site.collections = used
        .into_iter()
        .filter_map(|name| {
            let collection = input.routes.collection(name)?;
            Some(LintCollection::new(name, collection.permalink().as_str()))
        })
        .collect();
    for page in &content.pages {
        let front = &page.front;
        let named = (front.slug.as_ref(), front.title.as_deref());
        if let Some(slug) = slug_of(named, &page.path, &page.locale, config) {
            site.pages.push(PageRef::new(page.locale.clone(), slug));
        }
    }
    for entry in &content.glossary {
        let front = &entry.front;
        let named = (front.slug.as_ref(), front.term.as_deref());
        if let (Some(term), Some(slug)) = (
            &front.term,
            slug_of(named, &entry.path, &entry.locale, config),
        ) {
            site.glossary
                .push(GlossaryTerm::new(entry.locale.clone(), term, slug));
        }
    }
    site.locales = config.languages().to_vec();
    site.facts.clone_from(&input.facts);
    site.slugs = Some(config.slugs());
    let host = config.address().host();
    site.host = (!host.is_empty()).then(|| host.to_owned());
    // A process serves the one site `--site` names, so it has no sister
    // sites and `sibling_hosts` stays empty.
    (site, documents)
}

/// One post in the lint model, or `None` when no slug can be made for it or
/// its body does not render. Reading the site already refuses a broken
/// block, so a post that got this far renders.
fn lint_post(
    document: &Document<PostFront>,
    config: &Config,
    input: &LintInput,
) -> Option<LintPost> {
    let parts = PostParts {
        locale: &document.locale,
        path: &document.path,
        collection: &document.collection,
        front: &document.front,
        body: &document.body,
    };
    lint_parts(&parts, config, input)
}

/// A post that may not be a file under `content/` yet, such as a draft
/// about to be published: its language, where it is or will be, the
/// collection it is or will be in, its front matter and its markdown body.
pub(crate) struct PostParts<'a> {
    pub(crate) locale: &'a Locale,
    pub(crate) path: &'a Path,
    pub(crate) collection: &'a str,
    pub(crate) front: &'a PostFront,
    pub(crate) body: &'a str,
}

/// The post made of `parts` in the lint model, or `None` as for
/// [`lint_post`]. Its slug and address are the ones `khalid build` gives
/// its page, and its body is rendered with its wikilinks linked, so its
/// links are the page's links.
pub(crate) fn lint_parts(
    parts: &PostParts<'_>,
    config: &Config,
    input: &LintInput,
) -> Option<LintPost> {
    let front = parts.front;
    let named = (front.slug.as_ref(), front.title.as_deref());
    let slug = slug_of(named, parts.path, parts.locale, config)?;
    let none = Wikilinks::new();
    let links = input.links.get(parts.locale).unwrap_or(&none);
    let rendered = render_linked(parts.body, parts.locale, config.slugs(), links).ok()?;
    let address = PostAddress::new(parts.collection, parts.locale, &slug, front.date);
    let path = input.routes.address(config, &address).ok();
    let rest = routed(config, &input.routes, parts.locale, &address, parts.path).ok();
    let gaps = rest.map(|rest| gaps(parts, input, &rest, &slug));
    let brief = input.briefs.get(&(parts.locale.clone(), slug.clone()));
    let mut post = LintPost::new(parts.locale.clone(), slug);
    post.brief = brief.cloned();
    post.structured_data_gaps = gaps.unwrap_or_default();
    post.path = path;
    post.collection = Some(parts.collection.to_owned());
    post.title.clone_from(&front.title);
    post.seo_title.clone_from(&front.seo_title);
    post.description.clone_from(&front.description);
    post.date = front.date;
    post.updated = front.updated;
    post.reviewed = front.reviewed;
    post.author = front
        .author
        .as_ref()
        .map(|author| author.as_str().to_owned());
    post.translation.clone_from(&front.translation);
    post.aliases.clone_from(&front.aliases);
    post.focus_keyword.clone_from(&front.focus_keyword);
    post.secondary_keywords
        .clone_from(&front.secondary_keywords);
    post.cluster = front
        .cluster
        .as_ref()
        .map(cluster)
        .or_else(|| topic_group(front));
    post.wikilinks = wikilinks(parts.body);
    post.forms = forms(parts.body);
    let mut sections = section_shapes(&rendered.html).into_iter();
    post.headings = rendered
        .headings
        .iter()
        .filter_map(|h| {
            let holds = sections.next().unwrap_or_default();
            level(h.level).map(|level| Heading::new(level, h.text.clone()).holding(holds))
        })
        .collect();
    let (policy, own) = &input.policy;
    post.links = rendered
        .links
        .iter()
        .map(|link| lint_link(link, policy, own))
        .collect();
    post.images = rendered
        .images
        .iter()
        .map(|image| Image::new(image.src.clone(), image.alt.clone()))
        .collect();
    post.tables = rendered.html.matches("<table").count();
    post.reviews = rendered
        .structured
        .iter()
        .filter(|block| matches!(block, StructuredBlock::Review(_)))
        .count();
    post.word_count = rendered.words;
    post.body_text = plain_text(&rendered.html);
    Some(post)
}

/// The properties Google needs that the schema graph of the page `parts`
/// becomes lacks, its address ending in `rest` and its slug `slug`, as
/// `khalid-seo` checks the graph. A post with no date has no page yet, and
/// so no gaps.
fn gaps(
    parts: &PostParts<'_>,
    input: &LintInput,
    rest: &str,
    slug: &Slug,
) -> Vec<StructuredDataGap> {
    let (Some(site), Some(date)) = (&input.seo, parts.front.date) else {
        return Vec::new();
    };
    let front = parts.front;
    let title = front.title.clone().unwrap_or_else(|| slug.to_string());
    let Ok(mut page) = build_seo::post(parts.locale, (rest, &title), front, date) else {
        return Vec::new();
    };
    page.author = front.author.as_ref().map(Slug::to_string);
    build_seo::structured_data_gaps(site, &page)
}

/// The site, and the SEO page the post made of `parts` becomes on it, as
/// `khalid build` makes the page's head and schema graph: with the schema
/// of its blocks, its collection's schema templates and its hand-written
/// JSON-LD. Its author is named by slug, since only the build reads the
/// author files. `None` when the post has no date, no slug or no address,
/// does not render, or holds JSON-LD that is not schema.org, or when
/// `site.toml` gives no SEO site.
pub(crate) fn seo_page<'i>(
    parts: &PostParts<'_>,
    config: &Config,
    input: &'i LintInput,
) -> Option<(&'i SeoSite, SeoPage)> {
    let (site, front) = (input.seo.as_ref()?, parts.front);
    let named = (front.slug.as_ref(), front.title.as_deref());
    let slug = slug_of(named, parts.path, parts.locale, config)?;
    let none = Wikilinks::new();
    let links = input.links.get(parts.locale).unwrap_or(&none);
    let rendered = render_linked(parts.body, parts.locale, config.slugs(), links).ok()?;
    let address = PostAddress::new(parts.collection, parts.locale, &slug, front.date);
    let rest = routed(config, &input.routes, parts.locale, &address, parts.path).ok()?;
    let title = front.title.clone().unwrap_or_else(|| slug.to_string());
    let mut page = build_seo::post(parts.locale, (&rest, &title), front, front.date?).ok()?;
    page.collection = Some(parts.collection.to_owned());
    page.content_html = build_seo::full_addresses(config, &rendered.html);
    page.author = front.author.as_ref().map(Slug::to_string);
    let by = (path_of(config, parts.locale, &rest), page.author.clone());
    let body = (&rendered.structured[..], &rendered.links[..]);
    let by = (by.0.as_str(), by.1.as_deref().unwrap_or_default());
    build_schema::fill(&mut page, config, by, body).ok()?;
    page.custom_schema = build_schema::custom(parts.path, front.schema.as_deref()).ok()?;
    Some((site, page))
}

/// `link` in the lint model, with the `rel` words the link policy `policy`
/// gives it on the site `own`, as the built page has them. A rendered body
/// writes no `rel` of its own, so the policy's words are all of them.
fn lint_link(link: &Link, policy: &LinkPolicy, own: &SeoSite) -> LintLink {
    let kind = if link.internal {
        LinkKind::Internal
    } else {
        LinkKind::External
    };
    let mut model = LintLink::new(kind, link.href.clone(), link.text.clone());
    if let Some(rel) = policy.attributes_for(own, &link.href).rel() {
        model.rel = rel.split_whitespace().map(str::to_owned).collect();
    }
    model
}

/// A post's topic cluster from its header, in the lint model.
fn cluster(cluster: &TopicCluster) -> Cluster {
    let role = match cluster.role {
        TopicRole::Pillar => ClusterRole::Pillar,
        _ => ClusterRole::Member,
    };
    Cluster::new(cluster.name.as_str(), role)
}

/// A post's topic cluster from `topic_group` and `main_post` (P8), for a
/// post whose header names no `cluster`.
fn topic_group(front: &PostFront) -> Option<Cluster> {
    let role = if front.main_post {
        ClusterRole::Pillar
    } else {
        ClusterRole::Member
    };
    front
        .topic_group
        .as_deref()
        .map(|group| Cluster::new(group, role))
}

/// The slug a page is built at: from the header, or else from the title,
/// as `khalid build` names it, or else from the file name: `index.md` takes
/// its folder's name, and a leading `2026-09-14-` date is dropped.
fn slug_of(
    (slug, title): (Option<&Slug>, Option<&str>),
    path: &Path,
    locale: &Locale,
    config: &Config,
) -> Option<Slug> {
    if let Some(slug) = slug {
        return Some(slug.clone());
    }
    if let Some(slug) = title.and_then(|t| Slug::from_title(t, locale, config.slugs()).ok()) {
        return Some(slug);
    }
    let stem = path.file_stem()?.to_str()?;
    let stem = if stem == "index" {
        path.parent()?.file_name()?.to_str()?
    } else {
        stem
    };
    let undated = match (stem.get(..10), stem.get(10..11), stem.get(11..)) {
        (Some(day), Some("-"), Some(rest))
            if NaiveDate::parse_from_str(day, "%Y-%m-%d").is_ok() =>
        {
            rest
        }
        _ => stem,
    };
    Slug::from_title(undated, locale, config.slugs()).ok()
}

/// The forms of `body`, in order. No setting sends a form's answers
/// anywhere yet, so none has a destination.
fn forms(body: &str) -> Vec<LintForm> {
    let blocks = parse_blocks(body).unwrap_or_default();
    blocks
        .into_iter()
        .filter_map(|block| match block.kind {
            BlockKind::Form { id, .. } => Some(LintForm::new(id, Vec::new())),
            _ => None,
        })
        .collect()
}

/// The lint model's heading level for an HTML level from 1 to 6.
fn level(number: u8) -> Option<HeadingLevel> {
    match number {
        1 => Some(HeadingLevel::H1),
        2 => Some(HeadingLevel::H2),
        3 => Some(HeadingLevel::H3),
        4 => Some(HeadingLevel::H4),
        5 => Some(HeadingLevel::H5),
        6 => Some(HeadingLevel::H6),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use khalid_content::render;
    use khalid_core::{AbsoluteUrl, SlugMode};
    use khalid_seo::{LinkRel, LinkTarget};

    use super::*;

    #[test]
    fn a_link_carries_the_rel_words_the_link_policy_gives_its_page() {
        let mut policy = LinkPolicy::default();
        policy.external_rel.insert(LinkRel::NoFollow);
        policy.external_target = LinkTarget::NewTab;
        let english: Locale = "en".parse().unwrap();
        let address = AbsoluteUrl::parse("https://www.example.com/blog/").unwrap();
        let own = SeoSite::new("Example", address, english.clone());
        let body = "Read [the guide](/blog/link-text/), [the home page](https://example.com/) \
                    and [a study](https://research.example.org/links).";
        let rendered = render(body, &english, SlugMode::Ascii).unwrap();
        let rels: Vec<Vec<String>> = rendered
            .links
            .iter()
            .map(|link| lint_link(link, &policy, &own).rel)
            .collect();
        let none = Vec::<String>::new();
        assert_eq!(
            rels,
            [
                none.clone(),
                none,
                vec!["nofollow".to_owned(), "noopener".to_owned()]
            ]
        );
    }
}
