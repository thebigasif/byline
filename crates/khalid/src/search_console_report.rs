//! `khalid search-console report`: each post's numbers for the last 28
//! days Google has final numbers for, posts in decay, and the week's
//! winners and losers, as sentences or JSON.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use chrono::NaiveDate;
use khalid_content::SiteContent;
use khalid_core::{Config, Indexing};
use khalid_outbound::Outbound;
use khalid_search_console::{
    Change, Decay, PageNumbers, Question, SearchConsole, Window, decaying, go_deeper, movers,
};
use serde::Serialize;

use crate::bing::Failed;
use crate::cli::Output;
use crate::exit::Exit;
use crate::search_console::{ask, last_month};
use crate::search_console_lost::{self, LostBody};
use crate::{build_pages, index, report};

/// How many winners and how many losers a report lists.
const MOVERS: usize = 5;

/// A dated post readers can open, as Search Console names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LivePost {
    pub(crate) title: String,
    /// Its full address, such as `https://www.example.com/blog/a/`.
    pub(crate) address: String,
    /// Whether its front matter keeps it out of search results.
    pub(crate) noindex: bool,
}

/// The JSON shape of a report.
#[derive(Serialize)]
struct ReportBody<'a> {
    property: &'a str,
    first: String,
    last: String,
    posts: Vec<PostBody<'a>>,
    decaying: Vec<DecayBody>,
    lost: Vec<LostBody>,
    /// The last 7 days against the 7 before.
    winners: Vec<MoverBody>,
    losers: Vec<MoverBody>,
}

/// The JSON shape of one post's numbers. A post no search showed has no
/// position.
#[derive(Serialize)]
struct PostBody<'a> {
    title: &'a str,
    url: &'a str,
    clicks: f64,
    impressions: f64,
    ctr: f64,
    position: Option<f64>,
}

/// The JSON shape of one page's change in clicks.
#[derive(Serialize)]
struct MoverBody {
    title: String,
    url: String,
    before: f64,
    now: f64,
    /// Where to ask why it dropped, for a loser.
    go_deeper: Option<String>,
}

/// The JSON shape of a page whose clicks fell three months in a row.
#[derive(Serialize)]
struct DecayBody {
    title: String,
    url: String,
    /// Clicks in each 28-day month, oldest first.
    clicks: Vec<f64>,
    /// What to do about it.
    action: &'static str,
    go_deeper: String,
}

/// Reads the numbers of the checked site's posts from `console` up to
/// `today` and prints them.
pub(crate) fn run(
    (folder, config, content): (&Path, &Config, &SiteContent),
    (console, outbound, today): (&SearchConsole, &Outbound, NaiveDate),
    output: Output,
) -> Result<Exit, Failed> {
    let posts = live_posts(folder, config, content)?;
    let misses = search_console_lost::misses(folder)?;
    let month = last_month(today);
    let week = Window::ending(month.last, 7).unwrap_or(month);
    let before = week.before().unwrap_or(week);
    let earlier = |window: Window| window.before().unwrap_or(window);
    let (one, two) = (earlier(month), earlier(earlier(month)));
    let three = earlier(two);
    let (this_week, last_week, history) = ask(console, async {
        let session = console.sign_in(outbound).await?;
        let (now, then, h3, h2, h1, h0) = tokio::try_join!(
            session.pages(week),
            session.pages(before),
            session.pages(three),
            session.pages(two),
            session.pages(one),
            session.pages(month),
        )?;
        Ok((now, then, vec![h3, h2, h1, h0]))
    })?;
    let pages = history.last().map_or(&[][..], Vec::as_slice);
    let mut rows: Vec<PostBody<'_>> = posts.iter().map(|post| numbers(post, pages)).collect();
    rows.sort_by(|a, b| b.clicks.total_cmp(&a.clicks).then(a.title.cmp(b.title)));
    let week_movers = movers(&last_week, &this_week, MOVERS);
    let root = config.address().as_str().trim_end_matches('/');
    let body = ReportBody {
        lost: search_console_lost::lost(root, &posts, pages, &misses),
        property: console.property(),
        first: month.first.to_string(),
        last: month.last.to_string(),
        posts: rows,
        decaying: decaying(&history)
            .iter()
            .map(|decay| decay_body(&posts, decay))
            .collect(),
        winners: week_movers
            .winners
            .iter()
            .map(|c| mover(&posts, c, None))
            .collect(),
        losers: week_movers
            .losers
            .iter()
            .map(|c| mover(&posts, c, Some(Question::WhyItDropped)))
            .collect(),
    };
    Ok(match output {
        Output::Json => report::json(&body, Exit::Success),
        Output::Human => report::text(&sentences(&body), Exit::Success),
    })
}

/// Every dated post of the site, in the content index's order.
fn live_posts(
    folder: &Path,
    config: &Config,
    content: &SiteContent,
) -> Result<Vec<LivePost>, Failed> {
    let rows = index::of_content(folder, content).map_err(|message| (message, Exit::Failure))?;
    let hidden: BTreeSet<String> = content
        .posts
        .iter()
        .filter(|doc| matches!(&doc.front.robots, Some(r) if r.indexing == Indexing::NoIndex))
        .filter_map(|doc| index::slashed(&doc.path))
        .collect();
    let root = config.address().as_str().trim_end_matches('/');
    let posts = rows.posts.iter().filter(|row| row.date.is_some());
    Ok(posts
        .map(|row| LivePost {
            title: row.title.clone().unwrap_or_else(|| row.slug.to_string()),
            address: format!(
                "{root}{}",
                build_pages::path_of(config, &row.locale, &format!("{}/", row.slug))
            ),
            noindex: hidden.contains(&row.path),
        })
        .collect())
}

/// `post`'s numbers among `pages`, or none when no search showed it.
fn numbers<'a>(post: &'a LivePost, pages: &[PageNumbers]) -> PostBody<'a> {
    let found = pages.iter().find(|page| page.key == post.address);
    PostBody {
        title: &post.title,
        url: &post.address,
        clicks: found.map_or(0.0, |page| page.clicks),
        impressions: found.map_or(0.0, |page| page.impressions),
        ctr: found.map_or(0.0, |page| page.ctr),
        position: found.map(|page| page.position),
    }
}

/// `change` named by its post's title among `posts`, or by its address,
/// with a link that asks `question` when one is given.
fn mover(posts: &[LivePost], change: &Change, question: Option<Question>) -> MoverBody {
    let post = posts.iter().find(|post| post.address == change.key);
    MoverBody {
        title: post.map_or_else(|| change.key.clone(), |post| post.title.clone()),
        url: change.key.clone(),
        before: change.before,
        now: change.now,
        go_deeper: question.map(|question| go_deeper(question, &change.key)),
    }
}

/// `decay` named by its post's title among `posts`, or by its address.
fn decay_body(posts: &[LivePost], decay: &Decay) -> DecayBody {
    let post = posts.iter().find(|post| post.address == decay.key);
    DecayBody {
        title: post.map_or_else(|| decay.key.clone(), |post| post.title.clone()),
        url: decay.key.clone(),
        clicks: decay.clicks.clone(),
        action: "Refresh this post",
        go_deeper: go_deeper(Question::WhyItDropped, &decay.key),
    }
}

/// The report as sentences for a person.
fn sentences(body: &ReportBody<'_>) -> String {
    let mut text = format!(
        "Search Console numbers for {}, {} to {}:\n",
        body.property, body.first, body.last
    );
    if body.posts.is_empty() {
        text.push_str("The site has no published posts yet.\n");
    }
    for post in &body.posts {
        // Writing to a String cannot fail, so the results are dropped.
        let _ = match post.position {
            Some(position) => writeln!(
                text,
                "- {}: {:.0} clicks, {:.0} impressions, {:.1}% clicked, average position {position:.1}.",
                post.title,
                post.clicks,
                post.impressions,
                post.ctr * 100.0
            ),
            None => writeln!(text, "- {}: no search showed it.", post.title),
        };
        let _ = writeln!(text, "  {}", post.url);
    }
    if !body.decaying.is_empty() {
        text.push_str("\nThese lost clicks three months in a row. Refresh this post:\n");
    }
    for decay in &body.decaying {
        let months: Vec<String> = decay.clicks.iter().map(|c| format!("{c:.0}")).collect();
        let _ = writeln!(
            text,
            "- {}: {} clicks a month, oldest first.\n  Go deeper in SerpDad: {}",
            decay.title,
            months.join(", "),
            decay.go_deeper
        );
    }
    search_console_lost::sentences(&body.lost, &mut text);
    for (heading, list) in [
        ("gained the most clicks", &body.winners),
        ("lost the most clicks", &body.losers),
    ] {
        if !list.is_empty() {
            let _ = writeln!(text, "\nThis week against last week, these {heading}:");
        }
        for change in list {
            let _ = writeln!(
                text,
                "- {}: {:.0} clicks, from {:.0}.",
                change.title, change.now, change.before
            );
            if let Some(link) = &change.go_deeper {
                let _ = writeln!(text, "  Go deeper in SerpDad: {link}");
            }
        }
    }
    text
}
