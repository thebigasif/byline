//! Every post of the site as rows of the admin's posts table, each judged
//! by the lint publish gate against one view of the site.

use std::collections::BTreeMap;
use std::fs;

use khalid_api::content_store::{Listed, select};
use khalid_api::post_store::PostStoreError;
use khalid_api::query::ListQuery;
use khalid_api::resources::{Gate, LintCounts, PostRow, PostStatus};
use khalid_content::SiteContent;
use khalid_core::Slug;

use super::{DRAFTS, Found, Site, failure};

impl Site {
    /// One page of the posts as `query` asks: every draft, and every
    /// published post that has no draft of the same identity. The linter
    /// and the site view are made once. Every post gets the gate's quick
    /// answer, which only runs the blocking rules; only the rows on the
    /// page get every rule and their findings counted, because several
    /// warnings compare a post with every other post.
    pub(super) fn list_now(&self, query: &ListQuery) -> Result<Listed<PostRow>, PostStoreError> {
        let content = self.content()?;
        let judge = self.judge(&content)?;
        let mut rows = Vec::new();
        let mut files = BTreeMap::new();
        for found in self.every_post(&content)? {
            let post = Self::admin_post(&found)?;
            let gate = if judge
                .linter
                .refuses(&self.candidate_now(&found)?, &judge.site)
            {
                Gate::Refuses
            } else {
                Gate::Passes
            };
            rows.push(PostRow::new(&post, gate));
            files.insert(post.id().to_owned(), found);
        }
        let page = select(rows.iter(), query);
        let total = page.total();
        let mut counted = Vec::new();
        for row in page.into_items() {
            let found = files
                .get(row.id())
                .ok_or_else(|| failure("a listed post has no file"))?;
            let lint = LintCounts::of(&self.report(&judge, found)?);
            counted.push(row.with_lint(lint));
        }
        Ok(Listed::new(counted, total))
    }

    /// The drafts in `drafts/<language>/`, then the published posts with a
    /// slug in their front matter, which their identity needs.
    fn every_post(&self, content: &SiteContent) -> Result<Vec<Found>, PostStoreError> {
        let mut found = Vec::new();
        for locale in self.config.languages() {
            let mut folder = self.folder.clone();
            folder.push(format!("{DRAFTS}/{locale}"));
            let Ok(entries) = fs::read_dir(&folder) else {
                continue;
            };
            let mut names: Vec<String> = entries
                .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .collect();
            names.sort();
            for name in names {
                let slug = name.strip_suffix(".md").map(Slug::try_new_unicode);
                if let Some(Ok(slug)) = slug {
                    let path = self.draft_path(locale, &slug);
                    found.push(Found::read(locale.clone(), slug, path, PostStatus::Draft)?);
                }
            }
        }
        for doc in &content.posts {
            let Some(slug) = doc.front.slug.clone() else {
                continue;
            };
            if self.draft_path(&doc.locale, &slug).is_file() {
                continue;
            }
            let mut path = self.folder.clone();
            path.push(&doc.path);
            found.push(Found::read(
                doc.locale.clone(),
                slug,
                path,
                PostStatus::Published,
            )?);
        }
        Ok(found)
    }
}
