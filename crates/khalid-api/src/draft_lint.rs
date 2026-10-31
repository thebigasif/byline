//! What the create route answers beside a new draft: the lint publish
//! gate's findings, and where the draft opens in the admin, so a client
//! such as `SerpDad` can say "staged, 0 blocks, 2 warnings" and link to it
//! (KH-API-6, plan section 8).

use std::error::Error;

use khalid_lint::Report;
use serde::Serialize;
use utoipa::ToSchema;

use crate::post_store::PostStore;
use crate::resources::{AdminPost, LintCounts};

/// A new draft, as the create route answers it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CreatedPosts {
    /// The new draft, alone.
    posts: Vec<AdminPost>,
    /// What the linter finds in the draft as it would go live today, or
    /// `null` when this server runs no linter or could not run it now. The
    /// draft is saved either way.
    lint: Option<DraftLint>,
    /// Where the draft opens in the admin's editor, as a path from the
    /// server's root like the `Location` header, such as
    /// `/admin/posts/en.choosing-anchor-text`.
    admin_path: String,
}

/// The linter's verdict on a new draft.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DraftLint {
    /// How many findings would stop the draft going live, and how many
    /// are warnings.
    #[serde(flatten)]
    counts: LintCounts,
    /// Every finding, as `khalid lint --json` prints them for one post,
    /// and the post's `score` out of 100: the shape a refused publish
    /// carries under `lint`.
    #[schema(value_type = Object)]
    report: Report,
}

impl CreatedPosts {
    /// `post`, with what `posts` finds in it now. A lint that fails is
    /// logged and answered as `null`, since the draft is already saved and
    /// a retry with the same idempotency key lints it again.
    pub(crate) async fn linted(post: AdminPost, posts: &dyn PostStore) -> Self {
        let lint = match posts.lint(post.id()).await {
            Ok(report) => report.map(|report| DraftLint {
                counts: LintCounts::of(&report),
                report,
            }),
            Err(failed) => {
                let error: &(dyn Error + 'static) = &failed;
                tracing::error!(error, "a new draft could not be linted");
                None
            }
        };
        let admin_path = format!("/admin/posts/{}", post.id());
        Self {
            posts: vec![post],
            lint,
            admin_path,
        }
    }
}
