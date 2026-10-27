//! The dry-run report of a Ghost import: what it would bring in, what it
//! would map, and what it would leave out, in plain sentences.

use std::fmt;

use crate::ghost::{Dropped, Held, LeftReason, Plan};
use crate::sentences::{count, list};
use crate::{MapTarget, Place, SlugOrigin};

/// What a plan would do, counted, with the plan's lists for the details.
#[derive(Debug)]
#[non_exhaustive]
pub struct Report<'plan> {
    /// Published posts that come in.
    pub posts: usize,
    /// Drafts, scheduled posts and posts for members only.
    pub drafts: usize,
    /// Published pages.
    pub pages: usize,
    /// Public tags, whose archives are not mapped until posts hold tags.
    pub tags: usize,
    plan: &'plan Plan,
}

impl Plan {
    /// The report of this plan, which is the dry run's output.
    pub fn report(&self) -> Report<'_> {
        let placed = |place: Place| self.posts.iter().filter(|post| post.place == place).count();
        Report {
            posts: placed(Place::Post),
            drafts: placed(Place::Draft),
            pages: placed(Place::Page),
            tags: self.tags,
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
            "Coming in: {}, {}, {} and {}.",
            count(self.posts, "post", "posts"),
            count(self.drafts, "draft", "drafts"),
            count(self.pages, "page", "pages"),
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
            "Tag archives not mapped until posts can hold tags: {}.",
            self.tags
        )?;
        let holding = |wanted: fn(&Held) -> bool| {
            plan.posts
                .iter()
                .filter(|post| post.held.iter().any(wanted))
                .count()
        };
        writeln!(
            f,
            "Posts with fields their files cannot hold yet: {}, listed with each post.",
            holding(|_| true)
        )?;
        writeln!(
            f,
            "Posts for members only, kept as drafts so nothing paid is published: {}.",
            holding(|held| matches!(held, Held::MembersOnly(_)))
        )?;
        left_out(f, plan)
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
        "Slugs kept: {kept}. Slugs made for posts that had none: {made}."
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

/// What is never carried, what failed and what stays behind.
fn left_out(f: &mut fmt::Formatter<'_>, plan: &Plan) -> fmt::Result {
    let injected: Vec<String> = plan
        .posts
        .iter()
        .filter(|post| post.dropped.contains(&Dropped::CodeInjection))
        .map(|post| post.id.clone())
        .collect();
    list(
        f,
        "Posts with code injection, which is never carried",
        &injected,
    )?;
    list(f, "Cards with no place in Khalid, left out", &cards(plan))?;
    let failed: Vec<String> = plan
        .failed
        .iter()
        .map(|post| format!("{}, {}", post.id, post.error))
        .collect();
    list(f, "Posts that could not come in", &failed)?;
    let left: Vec<String> = plan
        .left_behind
        .iter()
        .map(|post| format!("{}, {}", post.id, left(&post.reason)))
        .collect();
    list(f, "Posts left behind", &left)
}

/// Each kind of card left out, in the order kinds first appear, with how
/// many posts had one.
fn cards(plan: &Plan) -> Vec<String> {
    let mut kinds: Vec<(&str, usize)> = Vec::new();
    let dropped = plan.posts.iter().flat_map(|post| post.dropped.iter());
    for kind in dropped
        .filter_map(|dropped| match dropped {
            Dropped::Cards(kinds) => Some(kinds),
            Dropped::CodeInjection => None,
        })
        .flatten()
    {
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

fn left(reason: &LeftReason) -> String {
    match reason {
        LeftReason::Newsletter => "a newsletter that was only ever emailed".to_owned(),
        LeftReason::UnpublishedPage => "a page that was never published".to_owned(),
        LeftReason::MembersPage(_) => {
            "a page for members only, which would be public here".to_owned()
        }
        LeftReason::Status(status) => {
            format!("a post marked {status}, which Khalid has no place for")
        }
    }
}
