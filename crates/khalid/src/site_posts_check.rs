//! The check behind the editor's SEO panel (G54): a post as it would go
//! live now, with the publish gate's findings and score, the search
//! result its head makes, and the schema graph it carries. Nothing is
//! written.

use std::path::PathBuf;

use khalid_api::content_store::BoxFuture;
use khalid_api::post_check::{PostCheck, PostChecks, Snippet};
use khalid_api::post_store::PostStoreError;
use khalid_api::resources::PostStatus;
use khalid_content::parse_post;
use khalid_seo::{HeadTag, SeoPage, SeoSite, head_tags, page_graph};
use serde_json::Value;

use super::{Found, Site, SitePosts, blocking, failure};
use crate::lint_input::{PostParts, seo_page};
use crate::post_text::PostText;

impl Site {
    /// `found`'s text as it would go live now, and its path in the site
    /// folder: a draft dated today in `content/`, a published post as it is.
    pub(super) fn as_now(&self, found: &Found) -> Result<(PostText, PathBuf), PostStoreError> {
        let mut text = found.text.clone();
        let relative = match found.status {
            PostStatus::Draft => self.dated(found, &mut text).1,
            _ => found
                .path
                .strip_prefix(&self.folder)
                .map_err(PostStoreError::new)?
                .to_path_buf(),
        };
        Ok((text, relative))
    }

    /// The check of post `id`, or `None` when no post has that identity.
    fn check_now(&self, id: &str) -> Result<Option<PostCheck>, PostStoreError> {
        let Some(found) = self.find(id)? else {
            return Ok(None);
        };
        let judge = self.judge(&self.content()?)?;
        let report = self.report(&judge, &found)?;
        let (text, relative) = self.as_now(&found)?;
        let post = parse_post(&relative, &text.to_text()).map_err(PostStoreError::new)?;
        let collection = relative.iter().nth(2).and_then(|part| part.to_str());
        let parts = PostParts {
            locale: &found.locale,
            path: &relative,
            collection: collection.unwrap_or("posts"),
            front: &post.front,
            body: &post.body,
        };
        let (site, page) = seo_page(&parts, &self.config, &judge.input)
            .ok_or_else(|| failure("the post's page cannot be made for its search preview"))?;
        let graph = page_graph(site, &page).map_err(PostStoreError::new)?;
        let schema: Value = serde_json::from_str(&graph.to_json()).map_err(PostStoreError::new)?;
        Ok(Some(PostCheck::new(
            id,
            report,
            snippet(site, &page)?,
            schema,
        )))
    }
}

/// The search result `page`'s head makes: its title, canonical address
/// and description.
fn snippet(site: &SeoSite, page: &SeoPage) -> Result<Snippet, PostStoreError> {
    let mut snippet = Snippet {
        title: String::new(),
        url: String::new(),
        description: String::new(),
    };
    let tags = head_tags(site, page).map_err(PostStoreError::new)?;
    for tag in tags.as_slice() {
        match tag {
            HeadTag::Title(title) => snippet.title.clone_from(title),
            HeadTag::Meta {
                name: "description",
                content,
            } => snippet.description.clone_from(content),
            HeadTag::Link {
                rel: "canonical",
                href,
            } => href.as_str().clone_into(&mut snippet.url),
            _ => {}
        }
    }
    Ok(snippet)
}

impl PostChecks for SitePosts {
    fn check<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<PostCheck>, PostStoreError>> {
        let id = id.to_owned();
        Box::pin(blocking(&self.0, move |site| site.check_now(&id)))
    }
}
