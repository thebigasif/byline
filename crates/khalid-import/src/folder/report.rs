//! The dry-run report of a markdown-folder import: what it would bring
//! in, what it would map, and what it would leave out, in plain sentences.

use std::fmt;

use crate::folder::Plan;
use crate::sentences::{count, list};
use crate::{MapTarget, Place, SlugOrigin};

/// What a plan would do, counted, with the plan's lists for the details.
#[derive(Debug)]
#[non_exhaustive]
pub struct Report<'plan> {
    /// Posts that come in.
    pub posts: usize,
    /// Pages that come in.
    pub pages: usize,
    /// Drafts that come in.
    pub drafts: usize,
    plan: &'plan Plan,
}

impl Plan {
    /// The report of this plan, which is the dry run's output.
    pub fn report(&self) -> Report<'_> {
        let placed = |place: Place| self.posts.iter().filter(|post| post.place == place).count();
        Report {
            posts: placed(Place::Post),
            pages: placed(Place::Page),
            drafts: placed(Place::Draft),
            plan: self,
        }
    }
}

impl fmt::Display for Report<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plan = self.plan;
        writeln!(f, "This is a dry run: nothing was written.")?;
        writeln!(
            f,
            "Coming in: {}, {} and {}.",
            count(self.posts, "post", "posts"),
            count(self.pages, "page", "pages"),
            count(self.drafts, "draft", "drafts"),
        )?;
        let changed: Vec<String> = plan
            .posts
            .iter()
            .filter_map(|post| match &post.slug_origin {
                SlugOrigin::Changed { was } => Some(format!("{was} to {}", post.slug)),
                SlugOrigin::Kept | SlugOrigin::Made => None,
            })
            .collect();
        writeln!(
            f,
            "Slugs kept: {}.",
            plan.posts.len().saturating_sub(changed.len())
        )?;
        if !changed.is_empty() {
            writeln!(f, "Slugs changed: {}.", changed.join("; "))?;
        }
        let same = plan
            .redirects
            .iter()
            .filter(|e| e.to == MapTarget::Kept)
            .count();
        writeln!(
            f,
            "Old addresses mapped: {}, of which {} move and {same} stay the same.",
            plan.redirects.len(),
            plan.redirects.len().saturating_sub(same),
        )?;
        let mut kinds: Vec<(&str, usize)> = Vec::new();
        for kind in plan.posts.iter().flat_map(|post| post.left.iter()) {
            match kinds.iter_mut().find(|(seen, _)| seen == kind) {
                Some((_, posts)) => *posts = posts.saturating_add(1),
                None => kinds.push((kind, 1)),
            }
        }
        let left: Vec<String> = kinds
            .into_iter()
            .map(|(kind, posts)| format!("{kind} ({})", count(posts, "file", "files")))
            .collect();
        list(f, "Left out", &left)?;
        let failed: Vec<String> = plan
            .failed
            .iter()
            .map(|post| format!("{}, {}", post.source, post.error))
            .collect();
        list(f, "Files that could not come in", &failed)?;
        let unread: Vec<String> = plan
            .unread
            .iter()
            .map(|file| format!("{}, {}", file.path, file.error))
            .collect();
        list(f, "Files that could not be read", &unread)
    }
}
