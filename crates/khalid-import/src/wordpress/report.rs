//! The dry-run report: what an import would bring in, what it would map,
//! and what it would leave behind, in plain sentences.

use std::fmt;

use crate::sentences::{count, list};
use crate::wordpress::{Held, LeftReason, Plan, SkipReason};
use crate::{MapTarget, Place, SlugOrigin};

/// What a plan would do, counted, with the plan's lists for the details.
#[derive(Debug)]
#[non_exhaustive]
pub struct Report<'plan> {
    /// Published posts that come in.
    pub posts: usize,
    /// Drafts, posts waiting for review and scheduled posts.
    pub drafts: usize,
    /// Published pages.
    pub pages: usize,
    /// Old addresses that move.
    pub redirects: usize,
    /// Old addresses that stay the same.
    pub kept_addresses: usize,
    plan: &'plan Plan,
}

impl Plan {
    /// The report of this plan, which is the dry run's output.
    pub fn report(&self) -> Report<'_> {
        let placed = |place: Place| self.items.iter().filter(|item| item.place == place).count();
        let kept = self
            .redirects
            .iter()
            .filter(|entry| entry.to == MapTarget::Kept)
            .count();
        Report {
            posts: placed(Place::Post),
            drafts: placed(Place::Draft),
            pages: placed(Place::Page),
            redirects: self.redirects.len().saturating_sub(kept),
            kept_addresses: kept,
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
            "Coming in: {}, {}, {}, {} and {}.",
            count(self.posts, "post", "posts"),
            count(self.drafts, "draft", "drafts"),
            count(self.pages, "page", "pages"),
            count(plan.authors, "author", "authors"),
            count(plan.categories, "category", "categories"),
        )?;
        let origin = |wanted: fn(&SlugOrigin) -> bool| {
            plan.items
                .iter()
                .filter(|item| wanted(&item.slug_origin))
                .count()
        };
        let (kept, made) = (
            origin(|o| *o == SlugOrigin::Kept),
            origin(|o| *o == SlugOrigin::Made),
        );
        writeln!(
            f,
            "Slugs kept: {kept}. Slugs made for posts that had none: {made}."
        )?;
        let changed: Vec<String> = plan
            .items
            .iter()
            .filter_map(|item| match &item.slug_origin {
                SlugOrigin::Changed { was } => Some(format!("{was} to {}", item.slug)),
                SlugOrigin::Kept | SlugOrigin::Made => None,
            })
            .collect();
        if !changed.is_empty() {
            writeln!(
                f,
                "Slugs changed to fit the site's rules: {}.",
                changed.join("; ")
            )?;
        }
        writeln!(
            f,
            "Old addresses mapped: {}, of which {} move and {} stay the same.",
            plan.redirects.len(),
            self.redirects,
            self.kept_addresses
        )?;
        let held: Vec<String> = plan
            .items
            .iter()
            .filter(|item| !item.held.is_empty())
            .map(|item| {
                let fields: Vec<&str> = item.held.iter().map(field).collect();
                format!("{} ({})", item.slug, fields.join(", "))
            })
            .collect();
        list(f, "Items with fields their files cannot hold yet", &held)?;
        let failed: Vec<String> = plan
            .failed
            .iter()
            .map(|item| format!("{}, {}", item.id, item.error))
            .collect();
        list(f, "Items that could not come in", &failed)?;
        list(f, "Items left behind", &grouped(plan))?;
        let rules: Vec<String> = plan
            .skipped_rules
            .iter()
            .map(|rule| format!("{} {}, {}", rule.id, rule.url, skipped(&rule.reason)))
            .collect();
        list(f, "Redirect rules left behind", &rules)
    }
}

/// The items left behind, one entry per reason in the order reasons first
/// appear, each with how many and the first few ids.
fn grouped(plan: &Plan) -> Vec<String> {
    /// How many ids an entry names before it says how many more there are.
    const SHOWN: usize = 3;
    let mut reasons: Vec<(&LeftReason, Vec<u64>)> = Vec::new();
    for item in &plan.left_behind {
        match reasons
            .iter_mut()
            .find(|(reason, _)| *reason == &item.reason)
        {
            Some((_, ids)) => ids.push(item.id),
            None => reasons.push((&item.reason, vec![item.id])),
        }
    }
    reasons
        .into_iter()
        .map(|(reason, ids)| {
            let shown: Vec<String> = ids.iter().take(SHOWN).map(u64::to_string).collect();
            let more = ids.len().saturating_sub(SHOWN);
            let tail = if more == 0 {
                String::new()
            } else {
                format!(" and {more} more")
            };
            format!(
                "{} ({}: {}{tail})",
                left(reason),
                ids.len(),
                shown.join(", ")
            )
        })
        .collect()
}

/// The name of a field an item holds back, as the report lists it.
fn field(held: &Held) -> &'static str {
    match held {
        Held::SearchTitle(_) => "search title",
        Held::Tags(_) => "tags",
        Held::FocusKeywords(_) => "focus keywords",
        Held::FeatureImage(_) => "feature image",
        Held::MoreCategories(_) => "more categories",
        Held::Shortcodes(_) => "shortcodes",
        Held::Unread(_) => "an SEO field it could not read",
        Held::Scheduled(_) => "scheduled time",
        Held::Robots(_) => "robots directives",
        Held::Canonical(_) => "canonical address",
    }
}

fn left(reason: &LeftReason) -> String {
    match reason {
        LeftReason::Attachment => "an uploaded file whose page leads to its post".to_owned(),
        LeftReason::Private => "private, seen only by editors".to_owned(),
        LeftReason::Trash => "in the bin".to_owned(),
        LeftReason::UnpublishedPage => "a page that was never published".to_owned(),
        LeftReason::Kind(kind) => format!("a {kind}, which Khalid has no place for"),
        LeftReason::Status(status) => {
            format!("a post marked {status}, which Khalid has no place for")
        }
    }
}

fn skipped(reason: &SkipReason) -> String {
    match reason {
        SkipReason::Disabled => "which is switched off".to_owned(),
        SkipReason::Conditional { match_type } => {
            format!("which depends on the visitor ({match_type})")
        }
        SkipReason::NotARedirect { action } => format!("which does not redirect ({action})"),
        SkipReason::Status { code } => {
            format!("which answers {code}, a status Khalid does not send")
        }
        SkipReason::AlreadyNotFound => "which answers 404 as Khalid already does".to_owned(),
        SkipReason::NoTarget => "which leads nowhere".to_owned(),
    }
}
