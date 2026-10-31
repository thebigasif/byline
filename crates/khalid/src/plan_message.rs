//! What `khalid plan` tells the person: what, why and fix (BL-ERR-4).

use chrono::NaiveDate;
use khalid_core::UserMessage;

/// Fewer posts went live in the last four weeks than in the four before.
pub(crate) fn pace_dropped(recent: usize, before: usize) -> UserMessage {
    UserMessage::new(
        format!(
            "{recent} {} went live in the last four weeks, against {before} in the four weeks before.",
            posts(recent)
        ),
        "Search engines read a steady publishing pace as a sign a site keeps covering its subject, and a drop weakens it.",
        "Publish the suggested drafts on their days to get back to your pace.",
    )
}

/// A member goes live before its cluster's pillar, or the pillar has no day.
pub(crate) fn member_first(member: &str, pillar: &str) -> UserMessage {
    UserMessage::new(
        format!("{member} is scheduled before {pillar}, the main post of its topic group."),
        "The post links to its main post, so readers and search engines meet a missing page.",
        format!("Move {member} to a day after {pillar} goes live, or publish {pillar} first."),
    )
}

/// No draft is ready, so the calendar ends after the last scheduled day.
pub(crate) fn none_ready(last: Option<NaiveDate>) -> UserMessage {
    let what = match last {
        Some(day) => format!("No draft is ready, so nothing goes live after {day}."),
        None => "No draft is ready and nothing is scheduled, so nothing new goes live.".to_owned(),
    };
    UserMessage::new(
        what,
        "A gap in publishing breaks the steady pace search engines read as a sign of an active site.",
        "Finish a draft until `khalid lint` passes it, then run `khalid plan` again.",
    )
}

/// A warning this version of `khalid plan` cannot put in words.
pub(crate) fn unknown() -> UserMessage {
    UserMessage::new(
        "The planner found something this version cannot describe.",
        "The plan above still holds.",
        "Update Khalid with `khalid update`, then run `khalid plan` again.",
    )
}

/// The posts or their schedules could not be read.
pub(crate) fn unreadable() -> UserMessage {
    UserMessage::new(
        "Khalid could not read the posts or their schedules.",
        "Without them it cannot say which draft goes out when.",
        "Check the site folder and data/site.db can be read, then run `khalid plan` again.",
    )
}

fn posts(count: usize) -> &'static str {
    if count == 1 { "post" } else { "posts" }
}
