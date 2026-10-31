//! `khalid search-console follow-ups`: the long, conversational searches
//! that showed each post in the last 28 days, the way people ask an AI
//! assistant, and whether a heading of the post answers each (GEO M3).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use chrono::NaiveDate;
use khalid_content::{SiteContent, render};
use khalid_core::Config;
use khalid_outbound::Outbound;
use khalid_search_console::{FOLLOW_UP_WORDS, SearchConsole, answered_by, follow_ups};
use serde::Serialize;

use crate::bing::Failed;
use crate::cli::Output;
use crate::exit::Exit;
use crate::search_console::{ask, last_month};
use crate::search_console_report::{LivePost, live_posts};
use crate::{index, report};

/// How many searches the text lists for one post. The JSON lists all.
const SHOWN: usize = 10;

/// The JSON shape of the finder's answer.
#[derive(Serialize)]
struct FollowUpsBody<'a> {
    property: &'a str,
    first: String,
    last: String,
    /// The fewest words a search had to have.
    words: usize,
    /// The posts the searches showed, the one with the most shown search
    /// first.
    posts: Vec<PostBody>,
}

/// The JSON shape of one post and its long searches.
#[derive(Serialize)]
struct PostBody {
    title: String,
    url: String,
    searches: Vec<SearchBody>,
}

/// The JSON shape of one long search.
#[derive(Serialize)]
struct SearchBody {
    query: String,
    clicks: f64,
    impressions: f64,
    position: f64,
    /// The heading that answers it, or none yet.
    answered_by: Option<String>,
}

/// Reads the long searches of the checked site's posts from `console` up
/// to `today` and prints them with the heading that answers each.
pub(crate) fn run(
    (folder, config, content): (&Path, &Config, &SiteContent),
    (console, outbound, today): (&SearchConsole, &Outbound, NaiveDate),
    output: Output,
) -> Result<Exit, Failed> {
    let posts = live_posts(folder, config, content)?;
    let month = last_month(today);
    let searches = ask(console, async {
        console.sign_in(outbound).await?.searches(month).await
    })?;
    let by_address: BTreeMap<&str, &LivePost> = posts
        .iter()
        .map(|post| (post.address.as_str(), post))
        .collect();
    let mut bodies: Vec<(PostBody, Vec<String>)> = Vec::new();
    let mut at: BTreeMap<&str, usize> = BTreeMap::new();
    for search in follow_ups(&searches, FOLLOW_UP_WORDS) {
        let Some(post) = by_address.get(search.page.as_str()) else {
            continue;
        };
        let place = *at.entry(&post.address).or_insert_with(|| {
            bodies.push((post_body(post), headings(config, content, post)));
            bodies.len().saturating_sub(1)
        });
        let Some((body, headings)) = bodies.get_mut(place) else {
            continue;
        };
        body.searches.push(SearchBody {
            answered_by: answered_by(&search.query, headings).map(str::to_owned),
            query: search.query,
            clicks: search.clicks,
            impressions: search.impressions,
            position: search.position,
        });
    }
    let body = FollowUpsBody {
        property: console.property(),
        first: month.first.to_string(),
        last: month.last.to_string(),
        words: FOLLOW_UP_WORDS,
        posts: bodies.into_iter().map(|(body, _)| body).collect(),
    };
    Ok(match output {
        Output::Json => report::json(&body, Exit::Success),
        Output::Human => report::text(&sentences(&body), Exit::Success),
    })
}

fn post_body(post: &LivePost) -> PostBody {
    PostBody {
        title: post.title.clone(),
        url: post.address.clone(),
        searches: Vec::new(),
    }
}

/// The headings of `post`'s body, in order, or none when its file is not
/// among the site's posts or its body does not render.
fn headings(config: &Config, content: &SiteContent, post: &LivePost) -> Vec<String> {
    let doc = content
        .posts
        .iter()
        .find(|doc| index::slashed(&doc.path).as_deref() == Some(post.path.as_str()));
    let Some(doc) = doc else {
        return Vec::new();
    };
    match render(&doc.body, &doc.locale, config.slugs()) {
        Ok(rendered) => rendered.headings.into_iter().map(|h| h.text).collect(),
        Err(why) => {
            tracing::warn!(path = %post.path, %why, "a post's headings could not be read");
            Vec::new()
        }
    }
}

/// The finder's answer as sentences for a person.
fn sentences(body: &FollowUpsBody<'_>) -> String {
    if body.posts.is_empty() {
        return format!(
            "No search of {} words or more showed your posts from {} to {}.\n\
             Google leaves out searches too rare to share, so a young or small site often has none yet.\n",
            body.words, body.first, body.last
        );
    }
    let searches = body.posts.iter().flat_map(|post| &post.searches);
    let open = searches
        .filter(|search| search.answered_by.is_none())
        .count();
    let mut text = format!(
        "Long searches, the way people ask an AI assistant, showed your posts from {} to {}.\n\
         {open} of them have no heading that answers them yet.\n\
         Each is a question readers bring to the post. A heading that asks it, with a 40 to 60 word answer under it, gives Google and AI answers a passage to quote.\n",
        body.first, body.last
    );
    for post in &body.posts {
        // Writing to a String cannot fail, so the results are dropped.
        let _ = write!(text, "\n{}\n{}\n", post.title, post.url);
        for search in post.searches.iter().take(SHOWN) {
            let answer = search.answered_by.as_deref().map_or_else(
                || "No heading answers it yet: add one that asks it.".to_owned(),
                |heading| format!("Answered under \"{heading}\"."),
            );
            let _ = writeln!(
                text,
                "- \"{}\": {:.0} impressions, {:.0} clicks, average position {:.1}. {answer}",
                search.query, search.impressions, search.clicks, search.position
            );
        }
        if let Some(more) = post.searches.len().checked_sub(SHOWN).filter(|n| *n > 0) {
            let _ = writeln!(text, "And {more} more, which --json lists.");
        }
    }
    text
}
