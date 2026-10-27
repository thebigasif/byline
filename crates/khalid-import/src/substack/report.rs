//! The dry-run report of a Substack import: what it would bring in, what
//! it would map, and what it would leave out, in plain sentences.

use std::fmt;

use crate::sentences::{count, list};
use crate::substack::Plan;
use crate::{MapTarget, Place, SlugOrigin};

/// What a plan would do, counted, with the plan's lists for the details.
#[derive(Debug)]
#[non_exhaustive]
pub struct Report<'plan> {
    /// Published posts that come in.
    pub posts: usize,
    /// Published pages that come in.
    pub pages: usize,
    /// Drafts that come in.
    pub drafts: usize,
    /// Posts that come in for signed-in members only.
    pub members: usize,
    /// Old addresses on substack.com, which only Substack answers.
    pub on_substack: usize,
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
            members: self.posts.iter().filter(|post| post.members).count(),
            on_substack: self.redirects.iter().filter(|e| e.on_substack()).count(),
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
        writeln!(
            f,
            "For signed-in members only, as on Substack: {}.",
            self.members
        )?;
        slugs(f, plan)?;
        let same = plan
            .redirects
            .iter()
            .filter(|entry| entry.to == MapTarget::Kept)
            .count();
        writeln!(
            f,
            "Old addresses mapped: {}, of which {} move and {same} stay the same.",
            plan.redirects.len(),
            plan.redirects.len().saturating_sub(same),
        )?;
        if self.on_substack > 0 {
            writeln!(
                f,
                "Old addresses on substack.com: {}. Substack cannot redirect them, so link each post there to its new home.",
                self.on_substack
            )?;
        }
        list(f, "Content with no place in Khalid, left out", &left(plan))?;
        let behind: Vec<String> = plan
            .left_behind
            .iter()
            .map(|post| format!("{}, {}", post.id, post.reason))
            .collect();
        list(f, "Posts left behind", &behind)?;
        let failed: Vec<String> = plan
            .failed
            .iter()
            .map(|post| format!("{}, {}", post.id, post.error))
            .collect();
        list(f, "Posts that could not come in", &failed)?;
        let unread: Vec<String> = plan
            .unread
            .iter()
            .map(|row| format!("line {}, {}", row.line, row.error))
            .collect();
        list(f, "Rows of posts.csv that could not be read", &unread)
    }
}

/// How many slugs were kept, and each one that changed.
fn slugs(f: &mut fmt::Formatter<'_>, plan: &Plan) -> fmt::Result {
    let kept = plan
        .posts
        .iter()
        .filter(|post| post.slug_origin == SlugOrigin::Kept)
        .count();
    let changed: Vec<String> = plan
        .posts
        .iter()
        .filter_map(|post| match &post.slug_origin {
            SlugOrigin::Changed { was } => Some(format!("{was} to {}", post.slug)),
            SlugOrigin::Kept | SlugOrigin::Made => None,
        })
        .collect();
    writeln!(f, "Slugs kept: {kept}.")?;
    if changed.is_empty() {
        return Ok(());
    }
    writeln!(
        f,
        "Slugs changed to fit the site's rules: {}.",
        changed.join("; ")
    )
}

/// Each kind of content left out, in the order kinds first appear, with
/// how many posts had one.
fn left(plan: &Plan) -> Vec<String> {
    let mut kinds: Vec<(&str, usize)> = Vec::new();
    for kind in plan.posts.iter().flat_map(|post| post.left.iter()) {
        match kinds.iter_mut().find(|(seen, _)| seen == kind) {
            Some((_, posts)) => *posts = posts.saturating_add(1),
            None => kinds.push((kind, 1)),
        }
    }
    kinds
        .into_iter()
        .map(|(kind, posts)| format!("{kind} ({})", count(posts, "post", "posts")))
        .collect()
}
