//! Turns the content files into the linter's own model (ADR 0019).
//!
//! The lint crate never reads files, so this binary maps parsed posts and
//! their rendered output to `LintPost` and `LintSite`, as the built page has
//! them: the slug and address `khalid build` gives each post, its body
//! rendered with its wikilinks linked, the wikilinks as written, its forms,
//! focus keywords, collection and topic cluster, the `rel` words the link
//! policy gives each link, the site's collections, and the links of each
//! home page, which lists every dated post (audit 2, G5), and the site's
//! fact registry (P7).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::NaiveDate;
use khalid_content::{
    BlockKind, Document, Link, PostFront, SiteContent, TopicCluster, TopicRole, Wikilinks,
    parse_blocks, render_linked,
};
use khalid_core::{Config, Locale, Slug};
use khalid_lint::{
    Cluster, ClusterRole, GlossaryTerm, Heading, HeadingLevel, Image, LinkKind, LintCollection,
    LintFact, LintForm, LintLink, LintPost, LintSite, PageRef, StructuredDataGap,
};
use khalid_routes::{PostAddress, Routes};
use khalid_seo::{LinkPolicy, SeoSite};

use crate::build_pages::routed;
use crate::build_seo;

/// What a post is linted against besides itself: the site's routes, the
/// pages a wikilink reaches in each language, and the site as the SEO
/// outputs read it, or `None` when `site.toml` cannot give one, in which
/// case `khalid build` stops with its own message.
pub(crate) struct LintInput {
    routes: Routes,
    links: BTreeMap<Locale, Wikilinks>,
    seo: Option<SeoSite>,
    policy: (LinkPolicy, SeoSite),
    facts: Vec<LintFact>,
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
        }
    }

    /// This input with the site's fact registry, read from `facts.toml`.
    pub(crate) fn with_facts(self, facts: Vec<LintFact>) -> Self {
        Self { facts, ..self }
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
    let mut post = LintPost::new(parts.locale.clone(), slug);
    post.structured_data_gaps = gaps.unwrap_or_default();
    post.path = path;
    post.collection = Some(parts.collection.to_owned());
    post.title.clone_from(&front.title);
    post.seo_title.clone_from(&front.seo_title);
    post.description.clone_from(&front.description);
    post.date = front.date;
    post.updated = front.updated;
    post.author = front
        .author
        .as_ref()
        .map(|author| author.as_str().to_owned());
    post.translation.clone_from(&front.translation);
    post.aliases.clone_from(&front.aliases);
    post.focus_keyword.clone_from(&front.focus_keyword);
    post.secondary_keywords
        .clone_from(&front.secondary_keywords);
    post.cluster = front.cluster.as_ref().map(cluster);
    post.wikilinks = wikilinks(parts.body);
    post.forms = forms(parts.body);
    post.headings = rendered
        .headings
        .iter()
        .filter_map(|h| level(h.level).map(|level| Heading::new(level, h.text.clone())))
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

/// The target of each `[[target]]` and `[[target|text]]` in `body`, as
/// written, in order. Code, fenced or inline, holds none.
fn wikilinks(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut fence: Option<&str> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (fence, marker) {
            (None, Some(marker)) => fence = Some(marker),
            (Some(open), Some(marker)) if open == marker => fence = None,
            (None, None) => {
                // Text between backticks is inline code: every other piece.
                for prose in line.split('`').step_by(2) {
                    let mut rest = prose;
                    while let Some((_, after)) = rest.split_once("[[") {
                        let Some((inside, tail)) = after.split_once("]]") else {
                            break;
                        };
                        let target = inside.split('|').next().unwrap_or_default().trim();
                        if !target.is_empty() {
                            targets.push(target.to_owned());
                        }
                        rest = tail;
                    }
                }
            }
            _ => {}
        }
    }
    targets
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

/// The elements whose end also ends a line of text, such as a heading or a
/// list item written without a full stop.
const BLOCKS: &[&str] = &[
    "p",
    "li",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "td",
    "th",
    "dt",
    "dd",
    "blockquote",
    "figcaption",
    "pre",
];

/// Whether the tag written `tag`, without its `<` and `>`, ends a line:
/// the end of a block, or a `br`.
fn ends_line(tag: &str) -> bool {
    let (closing, rest) = match tag.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, tag),
    };
    let name = rest.split([' ', '/']).next().unwrap_or_default();
    name.eq_ignore_ascii_case("br") || (closing && BLOCKS.contains(&name))
}

/// The words of rendered HTML with every tag and every entity, such as
/// `&quot;`, replaced by a space, so neither is read as a word. The end of
/// a block becomes a line break, so a heading never runs into the next
/// sentence. A formula's `MathML` is left out, so its letters and signs are
/// never read as words; a maths block's caption stays.
fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut tag = String::new();
    let mut skip_until = None;
    let mut in_maths = false;
    for c in html.chars() {
        match (skip_until, c) {
            (Some(end), _) if c == end => {
                skip_until = None;
                if end == '>' && tag == "/math" {
                    in_maths = false;
                } else if end == '>' && tag.split(' ').next() == Some("math") {
                    in_maths = true;
                }
                text.push(if end == '>' && ends_line(&tag) {
                    '\n'
                } else {
                    ' '
                });
                tag.clear();
            }
            (Some('>'), _) => tag.push(c),
            (Some(_), _) => {}
            (None, '<') => skip_until = Some('>'),
            (None, '&') => skip_until = Some(';'),
            (None, _) if in_maths => {}
            (None, _) => text.push(c),
        }
    }
    text
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

    #[test]
    fn plain_text_drops_tags_and_entities() {
        let html = "<p>Write a <em>&quot;title&quot;</em> tag &amp; test it.</p>";
        let text = plain_text(html);
        let words: Vec<&str> = text.split_whitespace().collect();
        assert_eq!(words, ["Write", "a", "title", "tag", "test", "it."]);
    }

    #[test]
    fn plain_text_ends_a_line_at_each_block_only() {
        let html = "<h2 id=\"why\">Why links</h2><ul><li>One <a href=\"/a/\">link</a></li><li>Two<br/>three</li></ul>";
        let lines: Vec<String> = plain_text(html)
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(lines, ["Why links", "One link", "Two", "three"]);
    }

    #[test]
    fn plain_text_leaves_out_formulas_and_keeps_their_captions() {
        let html = "<p>Growth <math><mi>x</mi></math> compounds.</p>\n<figure class=\"maths\">\n<math display=\"block\"><mi>A</mi><mo>=</mo><mi>P</mi></math>\n<figcaption>Compound growth</figcaption>\n</figure>\n";
        let words: Vec<String> = plain_text(html)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        assert_eq!(words, ["Growth", "compounds.", "Compound", "growth"]);
    }

    #[test]
    fn wikilinks_are_read_as_written_outside_code() {
        let body = "See [[why-links]] and [[link-text|good link text]].\n\n\
                    Write `[[not-a-link]]` like this.\n\n```\n[[in-a-fence]]\n```\n\n\
                    ~~~\n```\n[[still-fenced]]\n~~~\n\nLast [[ seo ]] and [[]] and [[open";
        assert_eq!(wikilinks(body), ["why-links", "link-text", "seo"]);
    }
}
