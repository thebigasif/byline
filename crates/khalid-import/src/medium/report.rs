//! The dry-run report of a Medium import: what it would bring in, what it
//! would map, and what it would leave out, in plain sentences.

use std::fmt;

use crate::medium::Plan;
use crate::sentences::{count, list};
use crate::{MapTarget, Place, SlugOrigin};

/// What a plan would do, counted, with the plan's lists for the details.
#[derive(Debug)]
#[non_exhaustive]
pub struct Report<'plan> {
    /// Published posts that come in.
    pub posts: usize,
    /// Drafts that come in.
    pub drafts: usize,
    /// Old addresses on medium.com, which only Medium answers.
    pub on_medium: usize,
    plan: &'plan Plan,
}

impl Plan {
    /// The report of this plan, which is the dry run's output.
    pub fn report(&self) -> Report<'_> {
        let placed = |place: Place| self.posts.iter().filter(|post| post.place == place).count();
        Report {
            posts: placed(Place::Post),
            drafts: placed(Place::Draft),
            on_medium: self
                .redirects
                .iter()
                .filter(|entry| entry.on_medium())
                .count(),
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
            count(self.drafts, "draft", "drafts"),
            count(plan.authors, "author", "authors"),
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
        writeln!(
            f,
            "Old addresses on medium.com: {}. Medium cannot redirect them, so link each post there to its new home, or unpublish it.",
            self.on_medium
        )?;
        list(f, "Content with no place in Khalid, left out", &left(plan))?;
        let failed: Vec<String> = plan
            .failed
            .iter()
            .map(|post| format!("{}, {}", post.id, post.error))
            .collect();
        list(f, "Posts that could not come in", &failed)?;
        let unread: Vec<String> = plan
            .unread
            .iter()
            .map(|file| format!("{}, {}", file.file, file.error))
            .collect();
        list(f, "Files that are not Medium posts", &unread)
    }
}

/// How many slugs were kept or made, and each one that changed.
fn slugs(f: &mut fmt::Formatter<'_>, plan: &Plan) -> fmt::Result {
    let origin = |wanted: SlugOrigin| {
        plan.posts
            .iter()
            .filter(|post| post.slug_origin == wanted)
            .count()
    };
    let (kept, made) = (origin(SlugOrigin::Kept), origin(SlugOrigin::Made));
    writeln!(
        f,
        "Slugs kept: {kept}. Slugs made for drafts that had none: {made}."
    )?;
    let changed: Vec<String> = plan
        .posts
        .iter()
        .filter_map(|post| match &post.slug_origin {
            SlugOrigin::Changed { was } => Some(format!("{was} to {}", post.slug)),
            SlugOrigin::Kept | SlugOrigin::Made => None,
        })
        .collect();
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
