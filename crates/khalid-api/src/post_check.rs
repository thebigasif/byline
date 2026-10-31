//! How a post reads to search engines before it goes live: what the linter
//! finds and its score, the snippet a search result shows, and the schema
//! graph the page carries (G54, KH-SEO-1, KH-SEO-4, KH-LNT-4).

use khalid_lint::Report;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::content_store::BoxFuture;
use crate::post_store::PostStoreError;

/// Checks posts as they would go live now, writing nothing. `khalid serve`
/// implements it with the publish gate's linter and the build's head and
/// schema; [`MemoryPostChecks`], from the `test-util` feature, answers
/// with checks made up front.
pub trait PostChecks: Send + Sync {
    /// The check of post `id`, or `None` when no post has that identity.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts or the site's settings
    /// cannot be read.
    fn check<'a>(&'a self, id: &'a str)
    -> BoxFuture<'a, Result<Option<PostCheck>, PostStoreError>>;
}

/// The lines of a search result: the title, the address and the
/// description, as the page's head gives them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Snippet {
    /// The `<title>`, after the site's title template.
    #[schema(example = "Choosing anchor text | Field Notes on SEO")]
    pub title: String,
    /// The canonical address.
    #[schema(example = "https://www.example.com/blog/choosing-anchor-text/")]
    pub url: String,
    /// The meta description, or empty when the page has none.
    #[schema(example = "Pick words that say where the link goes.")]
    pub description: String,
}

/// One post's check: findings and score, snippet and schema.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct PostCheck {
    /// The post's identity.
    id: String,
    /// Every finding, blocks first or not, as `khalid lint --json` prints
    /// them for one post, with `blocks` and the post's `score` out of 100
    /// and the rules that cost it points.
    #[schema(value_type = Object)]
    lint: Report,
    /// The search result the page would make.
    snippet: Snippet,
    /// The JSON-LD the page would carry: `@context` and `@graph`.
    #[schema(value_type = Object)]
    schema: Value,
}

impl PostCheck {
    /// The check of post `id`.
    pub fn new(id: &str, lint: Report, snippet: Snippet, schema: Value) -> Self {
        Self {
            id: id.to_owned(),
            lint,
            snippet,
            schema,
        }
    }

    /// The post's identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// What the linter finds, with the score.
    pub fn lint(&self) -> &Report {
        &self.lint
    }

    /// The search result the page would make.
    pub fn snippet(&self) -> &Snippet {
        &self.snippet
    }

    /// The JSON-LD the page would carry.
    pub fn schema(&self) -> &Value {
        &self.schema
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryPostChecks;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use super::{BoxFuture, PostCheck, PostChecks, PostStoreError};

    /// Checks made up front, for tests.
    #[derive(Debug, Default)]
    pub struct MemoryPostChecks {
        checks: Vec<PostCheck>,
    }

    impl MemoryPostChecks {
        /// Answers with `checks`, each for the post it names.
        pub fn new(checks: Vec<PostCheck>) -> Self {
            Self { checks }
        }
    }

    impl PostChecks for MemoryPostChecks {
        fn check<'a>(
            &'a self,
            id: &'a str,
        ) -> BoxFuture<'a, Result<Option<PostCheck>, PostStoreError>> {
            let found = self.checks.iter().find(|check| check.id == id).cloned();
            Box::pin(async move { Ok(found) })
        }
    }
}
