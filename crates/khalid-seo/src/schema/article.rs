//! The `Article` node of a post and the `Person` who wrote it.

use khalid_core::AbsoluteUrl;

use crate::schema::{image_node, node_id, publisher_id};
use crate::{
    ArticleType, AuthorProfile, EntityRole, SchemaNode, SchemaValue, SeoError, SeoPage, SeoSite,
};

/// Google shows at most this many characters of a headline.
const MAX_HEADLINE: usize = 110;

fn article_type(kind: ArticleType) -> &'static str {
    match kind {
        ArticleType::Article => "Article",
        ArticleType::BlogPosting => "BlogPosting",
        ArticleType::NewsArticle => "NewsArticle",
        ArticleType::TechArticle => "TechArticle",
    }
}

fn urls(list: &[AbsoluteUrl]) -> SchemaValue {
    SchemaValue::List(list.iter().cloned().map(SchemaValue::Url).collect())
}

/// The author's `Person` node: at `<author page>#person` when they have an
/// author page, else at `<post>#author`.
pub(crate) fn person_node(
    profile: &AuthorProfile,
    page_url: &AbsoluteUrl,
) -> Result<SchemaNode, SeoError> {
    let id = match &profile.url {
        Some(url) => node_id(url, "person")?,
        None => node_id(page_url, "author")?,
    };
    let mut node = SchemaNode::new("Person")
        .with_id(id)
        .with("name", SchemaValue::text(&profile.name));
    if let Some(url) = &profile.url {
        node.set("url", SchemaValue::Url(url.clone()));
    }
    if let Some(title) = &profile.job_title {
        node.set("jobTitle", SchemaValue::text(title));
    }
    if !profile.bio.is_empty() {
        node.set("description", SchemaValue::text(&profile.bio));
    }
    if let Some(image) = &profile.image {
        node.set("image", image_node(image));
    }
    if !profile.same_as.is_empty() {
        node.set("sameAs", urls(&profile.same_as));
    }
    if !profile.knows_about.is_empty() {
        let topics = profile
            .knows_about
            .iter()
            .map(|t| SchemaValue::text(t))
            .collect();
        node.set("knowsAbout", SchemaValue::List(topics));
    }
    Ok(node)
}

fn entities(page: &SeoPage, role: EntityRole) -> Option<SchemaValue> {
    let things: Vec<SchemaValue> = page
        .entities
        .iter()
        .filter(|entity| entity.role == role)
        .map(|entity| {
            let mut thing = SchemaNode::new("Thing").with("name", SchemaValue::text(&entity.name));
            if !entity.same_as.is_empty() {
                thing.set("sameAs", urls(&entity.same_as));
            }
            SchemaValue::Node(Box::new(thing))
        })
        .collect();
    (!things.is_empty()).then_some(SchemaValue::List(things))
}

/// The post's article node at `<post>#article`, then its author's `Person`
/// node when the post has an author.
pub(crate) fn article_nodes(
    site: &SeoSite,
    page: &SeoPage,
    page_id: AbsoluteUrl,
) -> Result<Vec<SchemaNode>, SeoError> {
    let page_url = site.url_for(&page.language, &page.path)?;
    let headline: String = page.title.chars().take(MAX_HEADLINE).collect();
    let mut article = SchemaNode::new(article_type(page.article_type))
        .with_id(node_id(&page_url, "article")?)
        .with(
            "headline",
            SchemaValue::Text(headline.trim_end().to_owned()),
        )
        .with("mainEntityOfPage", SchemaValue::Reference(page_id))
        .with("publisher", SchemaValue::Reference(publisher_id(site)?))
        .with("inLanguage", SchemaValue::text(page.language.as_str()));
    if !page.excerpt.is_empty() {
        article.set("description", SchemaValue::text(&page.excerpt));
    }
    let mut images: Vec<AbsoluteUrl> = page
        .share_image
        .iter()
        .map(|image| image.url.clone())
        .collect();
    images.extend(page.images.iter().cloned());
    if !images.is_empty() {
        article.set("image", urls(&images));
    }
    if let Some(published) = page.published {
        article.set("datePublished", SchemaValue::DateTime(published));
    }
    if let Some(modified) = page.modified.or(page.published) {
        article.set("dateModified", SchemaValue::DateTime(modified));
    }
    if let Some(words) = page.word_count {
        article.set("wordCount", SchemaValue::Integer(i64::from(words)));
    }
    if let Some(section) = &page.category {
        article.set("articleSection", SchemaValue::text(section));
    }
    for (property, role) in [
        ("about", EntityRole::About),
        ("mentions", EntityRole::Mention),
    ] {
        if let Some(things) = entities(page, role) {
            article.set(property, things);
        }
    }
    let profile = page
        .author_profile
        .clone()
        .or_else(|| page.author.as_deref().map(AuthorProfile::new));
    let mut nodes = Vec::with_capacity(2);
    let person = profile
        .map(|profile| person_node(&profile, &page_url))
        .transpose()?;
    if let Some(id) = person.as_ref().and_then(SchemaNode::id) {
        article.set("author", SchemaValue::Reference(id.clone()));
    }
    nodes.push(article);
    nodes.extend(person);
    Ok(nodes)
}
