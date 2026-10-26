//! The `Article` node of a post.

use khalid_core::AbsoluteUrl;

use crate::schema::{node_id, publisher_id};
use crate::{ArticleType, SchemaNode, SchemaValue, SeoError, SeoPage, SeoSite};

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

pub(crate) fn urls(list: &[AbsoluteUrl]) -> SchemaValue {
    SchemaValue::List(list.iter().cloned().map(SchemaValue::Url).collect())
}

/// The post's article node at `<post>#article`, typed by its content type.
pub(crate) fn article_node(
    site: &SeoSite,
    page: &SeoPage,
    page_id: AbsoluteUrl,
) -> Result<SchemaNode, SeoError> {
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
    if let Some(author) = &page.author {
        let person = SchemaNode::new("Person").with("name", SchemaValue::text(author));
        article.set("author", SchemaValue::Node(Box::new(person)));
    }
    Ok(article)
}
